use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use zapret_manager_telegram_transport::{
    parse_secret,
    relay::{parse_relay_token, RelayCredentials, RelayEndpoint},
    server::{probe_official_websocket, status_json, status_json_with_mode, TransportServer},
};
use zeroize::Zeroizing;

#[derive(Debug)]
struct Arguments {
    port: u16,
    secret_file: PathBuf,
    status_file: PathBuf,
    probe_dc: Option<u16>,
    media: bool,
    relay_endpoint_file: Option<PathBuf>,
    relay_token_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("event=transport_exit status=error reason={error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let arguments = parse_arguments(std::env::args_os().skip(1))?;
    if let Some(dc) = arguments.probe_dc {
        let endpoint = probe_official_websocket(dc, arguments.media)
            .await
            .map_err(|error| error.to_string())?;
        println!("event=probe status=ok endpoint={endpoint}");
        return Ok(());
    }

    let secret_text = Zeroizing::new(
        fs::read_to_string(&arguments.secret_file)
            .map_err(|error| format!("secret file could not be read: {error}"))?,
    );
    if secret_text.len() > 128 {
        return Err("secret file is too large".to_string());
    }
    let secret = parse_secret(secret_text.trim()).map_err(str::to_string)?;
    let relay = match (
        arguments.relay_endpoint_file.as_deref(),
        arguments.relay_token_file.as_deref(),
    ) {
        (Some(endpoint_file), Some(token_file)) => {
            let endpoint_text = read_bounded_text(endpoint_file, 2048, "relay endpoint")?;
            let token_text = read_bounded_text(token_file, 512, "relay token")?;
            let endpoint = RelayEndpoint::parse(endpoint_text.trim()).map_err(str::to_string)?;
            let token = parse_relay_token(token_text.trim()).map_err(str::to_string)?;
            Some(RelayCredentials::new(endpoint, token))
        }
        (None, None) => None,
        _ => return Err("relay endpoint and token files must be supplied together".to_string()),
    };
    let upstream_mode = if relay.is_some() {
        "user_relay"
    } else {
        "direct_official"
    };
    let server = match relay {
        Some(credentials) => TransportServer::bind_relay(arguments.port, secret, credentials).await,
        None => TransportServer::bind(arguments.port, secret).await,
    }
    .map_err(|error| error.to_string())?;
    let listen = server.local_addr().map_err(|error| error.to_string())?;
    write_status_atomic(
        &arguments.status_file,
        &if upstream_mode == "user_relay" {
            status_json_with_mode(listen, upstream_mode)
        } else {
            status_json(listen, &secret)
        }
        .map_err(|e| e.to_string())?,
    )?;
    eprintln!("event=transport_ready listen={listen}");
    server
        .run_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|error| error.to_string())
}

fn parse_arguments(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Arguments, String> {
    let mut port = 0u16;
    let mut secret_file = None;
    let mut status_file = None;
    let mut probe_dc = None;
    let mut media = false;
    let mut relay_endpoint_file = None;
    let mut relay_token_file = None;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "arguments must be valid Unicode".to_string())?;
        match argument.as_str() {
            "--port" => {
                port = next_value(&mut arguments, "--port")?
                    .parse()
                    .map_err(|_| "--port must be between 0 and 65535".to_string())?;
            }
            "--secret-file" => {
                secret_file = Some(PathBuf::from(next_value(&mut arguments, "--secret-file")?));
            }
            "--status-file" => {
                status_file = Some(PathBuf::from(next_value(&mut arguments, "--status-file")?));
            }
            "--probe-dc" => {
                probe_dc = Some(
                    next_value(&mut arguments, "--probe-dc")?
                        .parse()
                        .map_err(|_| "--probe-dc must be a number".to_string())?,
                );
            }
            "--media" => media = true,
            "--relay-endpoint-file" => {
                relay_endpoint_file = Some(PathBuf::from(next_value(
                    &mut arguments,
                    "--relay-endpoint-file",
                )?));
            }
            "--relay-token-file" => {
                relay_token_file = Some(PathBuf::from(next_value(
                    &mut arguments,
                    "--relay-token-file",
                )?));
            }
            _ => return Err(format!("unsupported argument: {argument}")),
        }
    }
    if probe_dc.is_none() && (secret_file.is_none() || status_file.is_none()) {
        return Err("--secret-file and --status-file are required".to_string());
    }
    if relay_endpoint_file.is_some() != relay_token_file.is_some() {
        return Err("relay endpoint and token files must be supplied together".to_string());
    }
    if probe_dc.is_some() && relay_endpoint_file.is_some() {
        return Err("relay files are not accepted in direct probe mode".to_string());
    }
    Ok(Arguments {
        port,
        secret_file: secret_file.unwrap_or_default(),
        status_file: status_file.unwrap_or_default(),
        probe_dc,
        media,
        relay_endpoint_file,
        relay_token_file,
    })
}

fn read_bounded_text(path: &Path, maximum: u64, label: &str) -> Result<Zeroizing<String>, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("{label} file could not be inspected: {error}"))?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(format!("{label} file is invalid"));
    }
    fs::read_to_string(path)
        .map(Zeroizing::new)
        .map_err(|error| format!("{label} file could not be read: {error}"))
}

fn next_value(
    arguments: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("missing value for {flag}"))?
        .into_string()
        .map_err(|_| format!("{flag} value must be valid Unicode"))
}

fn write_status_atomic(path: &Path, document: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "status file must have a parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("status directory could not be created: {error}"))?;
    let temporary = parent.join(format!(
        ".telegram-transport-status-{}.tmp",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("temporary status file could not be created: {error}"))?;
    file.write_all(document.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("status file could not be written: {error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("status file could not be committed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_reject_unknown_flags_and_require_files() {
        assert!(parse_arguments(["--unexpected".into()]).is_err());
        assert!(parse_arguments(["--port".into(), "1443".into()]).is_err());
        let parsed = parse_arguments([
            "--port".into(),
            "1443".into(),
            "--secret-file".into(),
            "secret.txt".into(),
            "--status-file".into(),
            "status.json".into(),
        ])
        .unwrap();
        assert_eq!(parsed.port, 1443);
    }

    #[test]
    fn probe_mode_does_not_accept_arbitrary_endpoints() {
        let parsed = parse_arguments(["--probe-dc".into(), "2".into()]).unwrap();
        assert_eq!(parsed.probe_dc, Some(2));
        assert!(parse_arguments(["--endpoint".into(), "example.com".into()]).is_err());
    }

    #[test]
    fn relay_files_are_opt_in_and_must_be_supplied_together() {
        let parsed = parse_arguments([
            "--secret-file".into(),
            "mtproxy-secret.txt".into(),
            "--status-file".into(),
            "status.json".into(),
            "--relay-endpoint-file".into(),
            "relay-endpoint.txt".into(),
            "--relay-token-file".into(),
            "relay-token.txt".into(),
        ])
        .unwrap();
        assert_eq!(
            parsed.relay_endpoint_file,
            Some(PathBuf::from("relay-endpoint.txt"))
        );
        assert_eq!(
            parsed.relay_token_file,
            Some(PathBuf::from("relay-token.txt"))
        );
        assert!(parse_arguments([
            "--secret-file".into(),
            "mtproxy-secret.txt".into(),
            "--status-file".into(),
            "status.json".into(),
            "--relay-endpoint-file".into(),
            "relay-endpoint.txt".into(),
        ])
        .is_err());
    }

    #[test]
    fn status_write_atomically_replaces_previous_owned_file() {
        let directory = tempfile::tempdir().unwrap();
        let status = directory.path().join("status.json");
        write_status_atomic(&status, "{\"state\":\"starting\"}").unwrap();
        write_status_atomic(&status, "{\"state\":\"ready\"}").unwrap();
        assert_eq!(
            std::fs::read_to_string(status).unwrap(),
            "{\"state\":\"ready\"}"
        );
    }
}
