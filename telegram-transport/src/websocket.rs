use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha1::{Digest, Sha1};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
    net::{lookup_host, TcpStream},
    time::timeout,
};
use tokio_rustls::{
    client::TlsStream,
    rustls::{self, pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WebSocketError {
    #[error("frame is too large")]
    FrameTooLarge,
    #[error("fragmented frames are not supported")]
    Fragmented,
    #[error("server frame must not be masked")]
    MaskedServerFrame,
    #[error("malformed websocket frame")]
    Malformed,
    #[error("websocket upgrade was rejected")]
    InvalidUpgrade,
    #[error("official Telegram endpoint did not resolve to a public address")]
    UnsafeDnsAnswer,
    #[error("upstream I/O failed: {0}")]
    Io(String),
    #[error("upstream operation timed out")]
    Timeout,
}

pub struct RawWebSocket {
    stream: TlsStream<TcpStream>,
    pending: Vec<u8>,
    closed: bool,
}

pub struct WebSocketReader<R> {
    reader: R,
    pending: Vec<u8>,
}

pub struct WebSocketWriter<W> {
    writer: W,
    closed: bool,
}

pub enum WebSocketMessage {
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong,
    Close,
}

impl RawWebSocket {
    pub async fn connect(domain: &str, connect_timeout: Duration) -> Result<Self, WebSocketError> {
        let addresses = timeout(connect_timeout, lookup_host((domain, 443)))
            .await
            .map_err(|_| WebSocketError::Timeout)?
            .map_err(|error| WebSocketError::Io(error.to_string()))?
            .filter(|address| is_official_telegram_ip(address.ip()))
            .take(8)
            .collect::<Vec<_>>();
        if addresses.is_empty() {
            return Err(WebSocketError::UnsafeDnsAnswer);
        }

        let mut tcp = None;
        for address in addresses {
            match timeout(connect_timeout, TcpStream::connect(address)).await {
                Ok(Ok(stream)) => {
                    tcp = Some(stream);
                    break;
                }
                Ok(Err(_)) | Err(_) => continue,
            }
        }
        let tcp = tcp.ok_or(WebSocketError::Timeout)?;
        tcp.set_nodelay(true)
            .map_err(|error| WebSocketError::Io(error.to_string()))?;

        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let server_name = ServerName::try_from(domain.to_owned())
            .map_err(|error| WebSocketError::Io(error.to_string()))?;
        let mut stream = timeout(
            connect_timeout,
            TlsConnector::from(Arc::new(config)).connect(server_name, tcp),
        )
        .await
        .map_err(|_| WebSocketError::Timeout)?
        .map_err(|error| WebSocketError::Io(error.to_string()))?;

        let mut nonce = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);
        let request_key = STANDARD.encode(nonce);
        let request = format!(
            "GET /apiws HTTP/1.1\r\n\
             Host: {domain}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {request_key}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Protocol: binary\r\n\
             \r\n"
        );
        timeout(connect_timeout, stream.write_all(request.as_bytes()))
            .await
            .map_err(|_| WebSocketError::Timeout)?
            .map_err(|error| WebSocketError::Io(error.to_string()))?;
        timeout(connect_timeout, stream.flush())
            .await
            .map_err(|_| WebSocketError::Timeout)?
            .map_err(|error| WebSocketError::Io(error.to_string()))?;

        const MAX_HEADERS: usize = 16 * 1024;
        let mut response = Vec::with_capacity(1024);
        let header_end = loop {
            if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
            if response.len() >= MAX_HEADERS {
                return Err(WebSocketError::InvalidUpgrade);
            }
            let mut buffer = [0u8; 1024];
            let read = timeout(connect_timeout, stream.read(&mut buffer))
                .await
                .map_err(|_| WebSocketError::Timeout)?
                .map_err(|error| WebSocketError::Io(error.to_string()))?;
            if read == 0 {
                return Err(WebSocketError::InvalidUpgrade);
            }
            response.extend_from_slice(&buffer[..read]);
        };
        let headers = std::str::from_utf8(&response[..header_end])
            .map_err(|_| WebSocketError::InvalidUpgrade)?;
        validate_upgrade_response(headers, &request_key)?;

        Ok(Self {
            stream,
            pending: response[header_end..].to_vec(),
            closed: false,
        })
    }

    pub fn split(
        self,
    ) -> (
        WebSocketReader<ReadHalf<TlsStream<TcpStream>>>,
        WebSocketWriter<WriteHalf<TlsStream<TcpStream>>>,
    ) {
        let (reader, writer) = tokio::io::split(self.stream);
        (
            WebSocketReader {
                reader,
                pending: self.pending,
            },
            WebSocketWriter {
                writer,
                closed: self.closed,
            },
        )
    }

    pub async fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let _ = send_control(&mut self.stream, 0x08, &[]).await;
        let _ = self.stream.shutdown().await;
    }
}

impl<R> WebSocketReader<R>
where
    R: AsyncRead + Unpin,
{
    pub async fn recv(&mut self) -> Result<WebSocketMessage, WebSocketError> {
        let mut header = [0u8; 2];
        self.read_exact(&mut header).await?;
        if header[0] & 0x80 == 0 {
            return Err(WebSocketError::Fragmented);
        }
        if header[1] & 0x80 != 0 {
            return Err(WebSocketError::MaskedServerFrame);
        }
        let length = match header[1] & 0x7f {
            length @ 0..=125 => length as usize,
            126 => {
                let mut bytes = [0u8; 2];
                self.read_exact(&mut bytes).await?;
                u16::from_be_bytes(bytes) as usize
            }
            127 => {
                let mut bytes = [0u8; 8];
                self.read_exact(&mut bytes).await?;
                usize::try_from(u64::from_be_bytes(bytes))
                    .map_err(|_| WebSocketError::FrameTooLarge)?
            }
            _ => unreachable!(),
        };
        if length > MAX_FRAME_BYTES {
            return Err(WebSocketError::FrameTooLarge);
        }
        let mut payload = vec![0u8; length];
        self.read_exact(&mut payload).await?;
        match header[0] & 0x0f {
            0x2 => Ok(WebSocketMessage::Binary(payload)),
            0x8 => Ok(WebSocketMessage::Close),
            0x9 => Ok(WebSocketMessage::Ping(payload)),
            0x0a => Ok(WebSocketMessage::Pong),
            _ => Err(WebSocketError::Malformed),
        }
    }

    async fn read_exact(&mut self, destination: &mut [u8]) -> Result<(), WebSocketError> {
        let copied = destination.len().min(self.pending.len());
        if copied > 0 {
            destination[..copied].copy_from_slice(&self.pending[..copied]);
            self.pending.drain(..copied);
        }
        if copied < destination.len() {
            self.reader
                .read_exact(&mut destination[copied..])
                .await
                .map_err(|error| WebSocketError::Io(error.to_string()))?;
        }
        Ok(())
    }
}

impl<W> WebSocketWriter<W>
where
    W: AsyncWrite + Unpin,
{
    pub async fn send_binary(&mut self, payload: &[u8]) -> Result<(), WebSocketError> {
        let mut mask = [0u8; 4];
        OsRng.fill_bytes(&mut mask);
        let frame = build_client_binary_frame(payload, mask)?;
        timeout(Duration::from_secs(30), self.writer.write_all(&frame))
            .await
            .map_err(|_| WebSocketError::Timeout)?
            .map_err(|error| WebSocketError::Io(error.to_string()))
    }

    pub async fn send_pong(&mut self, payload: &[u8]) -> Result<(), WebSocketError> {
        timeout(
            Duration::from_secs(30),
            send_control(&mut self.writer, 0x0a, payload),
        )
        .await
        .map_err(|_| WebSocketError::Timeout)?
    }

    pub async fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let _ = timeout(
            Duration::from_secs(5),
            send_control(&mut self.writer, 0x08, &[]),
        )
        .await;
        let _ = timeout(Duration::from_secs(5), self.writer.shutdown()).await;
    }
}

async fn send_control<W>(writer: &mut W, opcode: u8, payload: &[u8]) -> Result<(), WebSocketError>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > 125 {
        return Err(WebSocketError::Malformed);
    }
    let mut mask = [0u8; 4];
    OsRng.fill_bytes(&mut mask);
    let mut frame = Vec::with_capacity(payload.len() + 6);
    frame.push(0x80 | opcode);
    frame.push(0x80 | payload.len() as u8);
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % mask.len()]),
    );
    writer
        .write_all(&frame)
        .await
        .map_err(|error| WebSocketError::Io(error.to_string()))
}

pub fn build_client_binary_frame(payload: &[u8], mask: [u8; 4]) -> Result<Vec<u8>, WebSocketError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(WebSocketError::FrameTooLarge);
    }
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x82);
    match payload.len() {
        0..=125 => frame.push(0x80 | payload.len() as u8),
        126..=65535 => {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % mask.len()]),
    );
    Ok(frame)
}

#[cfg(test)]
pub fn parse_server_frame(frame: &[u8]) -> Result<(u8, Vec<u8>), WebSocketError> {
    if frame.len() < 2 {
        return Err(WebSocketError::Malformed);
    }
    if frame[0] & 0x80 == 0 {
        return Err(WebSocketError::Fragmented);
    }
    if frame[1] & 0x80 != 0 {
        return Err(WebSocketError::MaskedServerFrame);
    }
    let mut cursor = 2;
    let length = match frame[1] & 0x7f {
        length @ 0..=125 => length as usize,
        126 => {
            if frame.len() < cursor + 2 {
                return Err(WebSocketError::Malformed);
            }
            let length = u16::from_be_bytes([frame[cursor], frame[cursor + 1]]) as usize;
            cursor += 2;
            length
        }
        127 => {
            if frame.len() < cursor + 8 {
                return Err(WebSocketError::Malformed);
            }
            let length = u64::from_be_bytes(
                frame[cursor..cursor + 8]
                    .try_into()
                    .map_err(|_| WebSocketError::Malformed)?,
            );
            cursor += 8;
            usize::try_from(length).map_err(|_| WebSocketError::FrameTooLarge)?
        }
        _ => unreachable!(),
    };
    if length > MAX_FRAME_BYTES {
        return Err(WebSocketError::FrameTooLarge);
    }
    if frame.len() != cursor + length {
        return Err(WebSocketError::Malformed);
    }
    Ok((frame[0] & 0x0f, frame[cursor..].to_vec()))
}

pub fn validate_upgrade_response(response: &str, request_key: &str) -> Result<(), WebSocketError> {
    let mut lines = response.split("\r\n");
    let status = lines.next().ok_or(WebSocketError::InvalidUpgrade)?;
    if !status.starts_with("HTTP/1.1 101 ") && !status.starts_with("HTTP/1.0 101 ") {
        return Err(WebSocketError::InvalidUpgrade);
    }
    let mut upgrade = false;
    let mut connection = false;
    let mut accept = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket") {
            upgrade = true;
        } else if name.eq_ignore_ascii_case("connection")
            && value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
        {
            connection = true;
        } else if name.eq_ignore_ascii_case("sec-websocket-accept") {
            accept = Some(value);
        }
    }
    let expected = STANDARD.encode(
        Sha1::new()
            .chain_update(request_key.as_bytes())
            .chain_update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11")
            .finalize(),
    );
    if !upgrade || !connection || accept != Some(expected.as_str()) {
        return Err(WebSocketError::InvalidUpgrade);
    }
    Ok(())
}

#[cfg(test)]
pub fn is_public_upstream_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_public_ipv4(mapped);
            }
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !is_documentation_ipv6(ip)
        }
    }
}

pub fn is_official_telegram_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => [
            ([91, 108, 56, 0], 22),
            ([91, 108, 4, 0], 22),
            ([91, 108, 8, 0], 22),
            ([91, 108, 16, 0], 22),
            ([91, 108, 12, 0], 22),
            ([149, 154, 160, 0], 20),
            ([91, 105, 192, 0], 23),
            ([91, 108, 20, 0], 22),
            ([185, 76, 151, 0], 24),
        ]
        .into_iter()
        .any(|(network, prefix)| ipv4_in_prefix(ip, Ipv4Addr::from(network), prefix)),
        IpAddr::V6(ip) => [
            ("2001:b28:f23d::".parse().unwrap(), 48),
            ("2001:b28:f23f::".parse().unwrap(), 48),
            ("2001:67c:4e8::".parse().unwrap(), 48),
            ("2001:b28:f23c::".parse().unwrap(), 48),
            ("2a0a:f280::".parse().unwrap(), 32),
        ]
        .into_iter()
        .any(|(network, prefix)| ipv6_in_prefix(ip, network, prefix)),
    }
}

#[cfg(test)]
fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_unspecified()
        && !ip.is_loopback()
        && !ip.is_private()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !(octets[0] == 100 && (64..=127).contains(&octets[1]))
        && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
        && !(octets[0] == 198 && matches!(octets[1], 18 | 19))
        && !(octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
        && !(octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        && octets[0] < 240
}

#[cfg(test)]
fn is_documentation_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

fn ipv4_in_prefix(ip: Ipv4Addr, network: Ipv4Addr, prefix: u32) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
    u32::from(ip) & mask == u32::from(network) & mask
}

fn ipv6_in_prefix(ip: Ipv6Addr, network: Ipv6Addr, prefix: u32) -> bool {
    let mask = u128::MAX.checked_shl(128 - prefix).unwrap_or(0);
    u128::from(ip) & mask == u128::from(network) & mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_binary_frames_are_always_masked() {
        let frame = build_client_binary_frame(b"telegram", [1, 2, 3, 4]).unwrap();
        assert_eq!(&frame[..2], &[0x82, 0x88]);
        assert_eq!(&frame[2..6], &[1, 2, 3, 4]);
        assert_eq!(
            &frame[6..],
            &[
                b't' ^ 1,
                b'e' ^ 2,
                b'l' ^ 3,
                b'e' ^ 4,
                b'g' ^ 1,
                b'r' ^ 2,
                b'a' ^ 3,
                b'm' ^ 4,
            ]
        );
    }

    #[test]
    fn server_frame_parser_rejects_masking_fragmentation_and_oversize() {
        assert_eq!(
            parse_server_frame(&[0x02, 0x00]),
            Err(WebSocketError::Fragmented)
        );
        assert_eq!(
            parse_server_frame(&[0x82, 0x80, 1, 2, 3, 4]),
            Err(WebSocketError::MaskedServerFrame)
        );
        let too_large = (MAX_FRAME_BYTES as u64 + 1).to_be_bytes();
        let mut frame = vec![0x82, 127];
        frame.extend_from_slice(&too_large);
        assert_eq!(
            parse_server_frame(&frame),
            Err(WebSocketError::FrameTooLarge)
        );
    }

    #[test]
    fn server_binary_frame_round_trips() {
        assert_eq!(
            parse_server_frame(&[0x82, 3, 1, 2, 3]).unwrap(),
            (2, vec![1, 2, 3])
        );
    }

    #[test]
    fn upgrade_response_requires_101_and_matching_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let valid = concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Upgrade: websocket\r\n",
            "Connection: Upgrade\r\n",
            "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n",
            "\r\n"
        );
        assert!(validate_upgrade_response(valid, key).is_ok());
        assert!(validate_upgrade_response(
            &valid.replace("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", "invalid"),
            key
        )
        .is_err());
        assert!(validate_upgrade_response(
            &valid.replace("101 Switching Protocols", "302 Found"),
            key
        )
        .is_err());
    }

    #[test]
    fn upstream_dns_answers_must_be_public() {
        for rejected in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.0.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            assert!(
                !is_public_upstream_ip(rejected.parse().unwrap()),
                "{rejected}"
            );
        }
        for accepted in ["149.154.167.220", "91.108.56.100", "2001:67c:4e8:f002::a"] {
            assert!(
                is_public_upstream_ip(accepted.parse().unwrap()),
                "{accepted}"
            );
        }
    }

    #[test]
    fn upstream_dns_answers_must_match_official_telegram_cidrs() {
        for accepted in [
            "149.154.167.220",
            "91.108.56.100",
            "91.105.192.100",
            "2001:67c:4e8:f002::a",
        ] {
            assert!(
                is_official_telegram_ip(accepted.parse().unwrap()),
                "{accepted}"
            );
        }
        for rejected in ["1.1.1.1", "8.8.8.8", "127.0.0.1", "2001:4860:4860::8888"] {
            assert!(
                !is_official_telegram_ip(rejected.parse().unwrap()),
                "{rejected}"
            );
        }
    }
}
