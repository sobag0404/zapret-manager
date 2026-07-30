use crate::{
    official_ws_domains,
    protocol::{
        decode_client_handshake, BridgeCrypto, MessageSplitter, ProtocolError, HANDSHAKE_LEN,
    },
    relay::RelayCredentials,
    websocket::{RawWebSocket, WebSocketError, WebSocketMessage, MAX_RELAY_PAYLOAD_BYTES},
};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Semaphore},
    task::JoinSet,
    time::timeout,
};
use zeroize::Zeroizing;

pub const SOURCE_REVISION: &str = "21aaeb3aba97ad3b0ae39c6540a7b1afd12a3f7e";

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("transport I/O failed: {0}")]
    Io(String),
    #[error("client protocol was rejected: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("upstream websocket failed: {0}")]
    WebSocket(#[from] WebSocketError),
    #[error("transport operation timed out")]
    Timeout,
    #[error("status serialization failed: {0}")]
    Status(String),
}

pub struct TransportServer {
    listener: TcpListener,
    secret: Arc<Zeroizing<[u8; 16]>>,
    sessions: Arc<Semaphore>,
    upstream: Arc<UpstreamMode>,
}

enum UpstreamMode {
    DirectOfficial,
    UserRelay(RelayCredentials),
}

#[derive(Debug, Serialize)]
pub struct TransportStatus {
    pub state: &'static str,
    pub pid: u32,
    pub listen: SocketAddr,
    pub source_revision: &'static str,
    pub upstream_mode: &'static str,
}

impl TransportServer {
    pub async fn bind(port: u16, secret: [u8; 16]) -> Result<Self, TransportError> {
        Self::bind_with_upstream(port, secret, UpstreamMode::DirectOfficial).await
    }

    pub async fn bind_relay(
        port: u16,
        secret: [u8; 16],
        credentials: RelayCredentials,
    ) -> Result<Self, TransportError> {
        Self::bind_with_upstream(port, secret, UpstreamMode::UserRelay(credentials)).await
    }

    async fn bind_with_upstream(
        port: u16,
        secret: [u8; 16],
        upstream: UpstreamMode,
    ) -> Result<Self, TransportError> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|error| TransportError::Io(error.to_string()))?;
        Ok(Self {
            listener,
            secret: Arc::new(Zeroizing::new(secret)),
            sessions: Arc::new(Semaphore::new(8)),
            upstream: Arc::new(upstream),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.listener
            .local_addr()
            .map_err(|error| TransportError::Io(error.to_string()))
    }

    pub async fn run_until<F>(self, shutdown: F) -> Result<(), TransportError>
    where
        F: Future<Output = ()>,
    {
        let Self {
            listener,
            secret,
            sessions,
            upstream,
        } = self;
        let mut tasks = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                completed = tasks.join_next(), if !tasks.is_empty() => {
                    let _ = completed;
                }
                accepted = listener.accept() => {
                    let (stream, _) =
                        accepted.map_err(|error| TransportError::Io(error.to_string()))?;
                    let Ok(permit) = sessions.clone().try_acquire_owned() else {
                        drop(stream);
                        continue;
                    };
                    let secret = secret.clone();
                    let upstream = upstream.clone();
                    tasks.spawn(async move {
                        let result = handle_client(stream, secret.as_ref(), upstream.as_ref()).await;
                        drop(permit);
                        if let Err(error) = result {
                            eprintln!("event=session_closed status=error reason={}", error_code(&error));
                        }
                    });
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

pub fn status_json(listen: SocketAddr, secret: &[u8; 16]) -> Result<String, TransportError> {
    let _ = secret;
    status_json_with_mode(listen, "direct_official")
}

pub fn status_json_with_mode(
    listen: SocketAddr,
    upstream_mode: &'static str,
) -> Result<String, TransportError> {
    serde_json::to_string(&TransportStatus {
        state: "ready",
        pid: std::process::id(),
        listen,
        source_revision: SOURCE_REVISION,
        upstream_mode,
    })
    .map_err(|error| TransportError::Status(error.to_string()))
}

pub async fn probe_official_websocket(dc: u16, media: bool) -> Result<String, TransportError> {
    let domains = official_ws_domains(&crate::TelegramTarget { dc, media })
        .map_err(|_| TransportError::Protocol(ProtocolError::UnsupportedDataCenter))?;
    let mut last_error = None;
    for domain in domains {
        match RawWebSocket::connect(&domain, Duration::from_secs(10)).await {
            Ok(mut websocket) => {
                websocket.close().await;
                return Ok(domain);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(TransportError::WebSocket(
        last_error.unwrap_or(WebSocketError::Timeout),
    ))
}

async fn handle_client(
    mut stream: TcpStream,
    secret: &[u8; 16],
    upstream: &UpstreamMode,
) -> Result<(), TransportError> {
    stream
        .set_nodelay(true)
        .map_err(|error| TransportError::Io(error.to_string()))?;
    let mut handshake = [0u8; HANDSHAKE_LEN];
    timeout(Duration::from_secs(3), stream.read_exact(&mut handshake))
        .await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|error| TransportError::Io(error.to_string()))?;
    let client = decode_client_handshake(&handshake, secret)?;
    let relay_random = generate_relay_random();
    let (crypto, relay_init) = BridgeCrypto::from_client(&client, secret, relay_random)?;
    let mut splitter = MessageSplitter::from_relay(&relay_init, client.protocol)?;
    let relay_mode = matches!(upstream, UpstreamMode::UserRelay(_));

    let (websocket, endpoint_label) = match upstream {
        UpstreamMode::DirectOfficial => {
            let domains = official_ws_domains(&client.target)
                .map_err(|_| TransportError::Protocol(ProtocolError::UnsupportedDataCenter))?;
            let mut websocket = None;
            let mut last_error = None;
            for domain in domains {
                match RawWebSocket::connect(&domain, Duration::from_secs(10)).await {
                    Ok(connected) => {
                        websocket = Some(connected);
                        break;
                    }
                    Err(error) => last_error = Some(error),
                }
            }
            (
                websocket.ok_or_else(|| {
                    TransportError::WebSocket(last_error.unwrap_or(WebSocketError::Timeout))
                })?,
                "official",
            )
        }
        UpstreamMode::UserRelay(credentials) => (
            RawWebSocket::connect_relay(credentials, &client.target, Duration::from_secs(10))
                .await?,
            "user_relay",
        ),
    };
    let (mut websocket_reader, mut websocket_writer) = websocket.split();
    websocket_writer.send_binary(&relay_init).await?;
    eprintln!(
        "event=upstream_connected dc={} media={} endpoint={}",
        client.target.dc, client.target.media, endpoint_label
    );

    let (mut outbound_crypto, mut inbound_crypto) = crypto.split();
    let (mut local_reader, mut local_writer) = stream.into_split();
    let (commands, mut command_receiver) = mpsc::channel::<WebSocketCommand>(32);
    let (acknowledgements, mut acknowledgement_receiver) = mpsc::channel::<u32>(1);

    let outbound_commands = commands.clone();
    let outbound = async move {
        let mut local_buffer = vec![0u8; 64 * 1024];
        loop {
            let read = timeout(
                Duration::from_secs(120),
                local_reader.read(&mut local_buffer),
            )
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|error| TransportError::Io(error.to_string()))?;
            if read == 0 {
                let _ = outbound_commands.send(WebSocketCommand::Close).await;
                return Ok(());
            }
            let encoded = outbound_crypto.transform(&local_buffer[..read]);
            for packet in splitter.push(&encoded)? {
                enqueue_binary_packet(&outbound_commands, packet, relay_mode).await?;
            }
        }
    };

    let inbound_commands = commands.clone();
    let inbound = async move {
        loop {
            let message = timeout(Duration::from_secs(120), websocket_reader.recv())
                .await
                .map_err(|_| TransportError::Timeout)??;
            match message {
                WebSocketMessage::Binary(packet) => {
                    let local = inbound_crypto.transform(&packet);
                    timeout(Duration::from_secs(30), local_writer.write_all(&local))
                        .await
                        .map_err(|_| TransportError::Timeout)?
                        .map_err(|error| TransportError::Io(error.to_string()))?;
                }
                WebSocketMessage::RelayBinary { payload, sequence } => {
                    let local = inbound_crypto.transform(&payload);
                    timeout(Duration::from_secs(30), local_writer.write_all(&local))
                        .await
                        .map_err(|_| TransportError::Timeout)?
                        .map_err(|error| TransportError::Io(error.to_string()))?;
                    acknowledgements
                        .send(sequence)
                        .await
                        .map_err(|_| TransportError::Io("websocket writer stopped".to_string()))?;
                }
                WebSocketMessage::Ping(payload) => {
                    inbound_commands
                        .send(WebSocketCommand::Pong(payload))
                        .await
                        .map_err(|_| TransportError::Io("websocket writer stopped".to_string()))?;
                }
                WebSocketMessage::Pong => {}
                WebSocketMessage::Close => return Ok(()),
            }
        }
    };

    drop(commands);
    let writer = async move {
        while let Some(command) =
            next_writer_command(&mut acknowledgement_receiver, &mut command_receiver).await
        {
            match command {
                WebSocketCommand::Binary(packet) => {
                    websocket_writer.send_binary(&packet).await?;
                }
                WebSocketCommand::Pong(payload) => {
                    websocket_writer.send_pong(&payload).await?;
                }
                WebSocketCommand::RelayAck(sequence) => {
                    websocket_writer.send_relay_ack(sequence).await?;
                }
                WebSocketCommand::Close => {
                    websocket_writer.close().await;
                    return Ok(());
                }
            }
        }
        websocket_writer.close().await;
        Ok(())
    };

    let mut outbound = Box::pin(outbound);
    let mut inbound = Box::pin(inbound);
    let mut writer = Box::pin(writer);
    let first = tokio::select! {
        result = &mut outbound => SessionCompletion::Producer(result),
        result = &mut inbound => SessionCompletion::Producer(result),
        result = &mut writer => SessionCompletion::Writer(result),
    };
    match first {
        SessionCompletion::Writer(result) => {
            drop(outbound);
            drop(inbound);
            result
        }
        SessionCompletion::Producer(result) => {
            drop(outbound);
            drop(inbound);
            let writer_result = timeout(Duration::from_secs(10), writer.as_mut())
                .await
                .map_err(|_| TransportError::Timeout)?;
            result.and(writer_result)
        }
    }
}

enum WebSocketCommand {
    Binary(Vec<u8>),
    Pong(Vec<u8>),
    RelayAck(u32),
    Close,
}

async fn enqueue_binary_packet(
    commands: &mpsc::Sender<WebSocketCommand>,
    packet: Vec<u8>,
    relay_mode: bool,
) -> Result<(), TransportError> {
    if relay_mode {
        for chunk in packet.chunks(MAX_RELAY_PAYLOAD_BYTES) {
            commands
                .send(WebSocketCommand::Binary(chunk.to_vec()))
                .await
                .map_err(|_| TransportError::Io("websocket writer stopped".to_string()))?;
        }
    } else {
        commands
            .send(WebSocketCommand::Binary(packet))
            .await
            .map_err(|_| TransportError::Io("websocket writer stopped".to_string()))?;
    }
    Ok(())
}

async fn next_writer_command(
    acknowledgements: &mut mpsc::Receiver<u32>,
    commands: &mut mpsc::Receiver<WebSocketCommand>,
) -> Option<WebSocketCommand> {
    tokio::select! {
        biased;
        Some(sequence) = acknowledgements.recv() => Some(WebSocketCommand::RelayAck(sequence)),
        command = commands.recv() => command,
    }
}

enum SessionCompletion {
    Producer(Result<(), TransportError>),
    Writer(Result<(), TransportError>),
}

fn generate_relay_random() -> [u8; HANDSHAKE_LEN] {
    const RESERVED: [[u8; 4]; 6] = [
        *b"HEAD",
        *b"POST",
        *b"GET ",
        [0xee; 4],
        [0xdd; 4],
        [0x16, 0x03, 0x01, 0x02],
    ];
    loop {
        let mut bytes = [0u8; HANDSHAKE_LEN];
        OsRng.fill_bytes(&mut bytes);
        if bytes[0] == 0xef
            || RESERVED.contains(&bytes[..4].try_into().unwrap_or([0u8; 4]))
            || bytes[4..8] == [0u8; 4]
        {
            continue;
        }
        return bytes;
    }
}

fn error_code(error: &TransportError) -> &'static str {
    match error {
        TransportError::Io(_) => "io",
        TransportError::Protocol(_) => "protocol",
        TransportError::WebSocket(_) => "websocket",
        TransportError::Timeout => "timeout",
        TransportError::Status(_) => "status",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_binds_only_ipv4_loopback() {
        let server = TransportServer::bind(0, [0x11; 16]).await.unwrap();
        let address = server.local_addr().unwrap();
        assert_eq!(address.ip().to_string(), "127.0.0.1");
        assert_ne!(address.port(), 0);
    }

    #[test]
    fn status_document_has_provenance_but_no_secret() {
        let secret = [0xabu8; 16];
        let document = status_json("127.0.0.1:15555".parse().unwrap(), &secret).unwrap();
        assert!(document.contains("\"state\":\"ready\""));
        assert!(document.contains("\"listen\":\"127.0.0.1:15555\""));
        assert!(
            document.contains("\"source_revision\":\"21aaeb3aba97ad3b0ae39c6540a7b1afd12a3f7e\"")
        );
        assert!(!document.contains("abababababababababababababababab"));
    }

    #[test]
    fn relay_status_discloses_mode_but_not_endpoint_or_token() {
        let document =
            status_json_with_mode("127.0.0.1:15555".parse().unwrap(), "user_relay").unwrap();
        assert!(document.contains("\"upstream_mode\":\"user_relay\""));
        assert!(!document.contains("workers.dev"));
        assert!(!document.contains("Authorization"));
        assert!(!document.contains("token"));
    }

    #[tokio::test]
    async fn relay_ack_has_priority_over_a_full_data_queue() {
        let (command_sender, mut commands) = mpsc::channel(32);
        for _ in 0..32 {
            command_sender
                .send(WebSocketCommand::Binary(vec![1]))
                .await
                .unwrap();
        }
        let (ack_sender, mut acknowledgements) = mpsc::channel(1);
        ack_sender.send(42).await.unwrap();
        assert!(matches!(
            next_writer_command(&mut acknowledgements, &mut commands).await,
            Some(WebSocketCommand::RelayAck(42))
        ));
    }

    #[tokio::test]
    async fn relay_packets_are_enqueued_as_bounded_writer_commands() {
        let (sender, mut receiver) = mpsc::channel(4);
        enqueue_binary_packet(&sender, vec![7; MAX_RELAY_PAYLOAD_BYTES * 2 + 1], true)
            .await
            .unwrap();
        let mut lengths = Vec::new();
        for _ in 0..3 {
            let Some(WebSocketCommand::Binary(chunk)) = receiver.recv().await else {
                panic!("expected relay binary command");
            };
            lengths.push(chunk.len());
        }
        assert_eq!(
            lengths,
            vec![MAX_RELAY_PAYLOAD_BYTES, MAX_RELAY_PAYLOAD_BYTES, 1]
        );
    }
}
