use crate::TelegramTarget;
use url::{Host, Url};
use zeroize::Zeroizing;

pub const RELAY_PROTOCOL: &str = "zm-telegram-relay-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayEndpoint {
    host: String,
}

impl RelayEndpoint {
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let url = Url::parse(value).map_err(|_| "relay endpoint must be a valid wss URL")?;
        if url.scheme() != "wss" {
            return Err("relay endpoint must use wss");
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err("relay endpoint must not contain credentials");
        }
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err("relay endpoint must be a bare origin");
        }
        if url.port_or_known_default() != Some(443) || url.port().is_some_and(|port| port != 443) {
            return Err("relay endpoint must use TCP port 443");
        }
        let Host::Domain(host) = url.host().ok_or("relay endpoint must have a hostname")? else {
            return Err("relay endpoint must use a DNS hostname");
        };
        let normalized = host.trim_end_matches('.').to_ascii_lowercase();
        if normalized == "localhost"
            || normalized.ends_with(".localhost")
            || normalized.ends_with(".local")
            || !normalized.contains('.')
        {
            return Err("relay endpoint hostname is not allowed");
        }
        Ok(Self { host: normalized })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        443
    }
}

#[derive(Clone)]
pub struct RelayCredentials {
    endpoint: RelayEndpoint,
    token: Zeroizing<String>,
}

impl RelayCredentials {
    pub fn new(endpoint: RelayEndpoint, token: Zeroizing<String>) -> Self {
        Self { endpoint, token }
    }

    pub fn endpoint(&self) -> &RelayEndpoint {
        &self.endpoint
    }

    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

pub fn parse_relay_token(value: &str) -> Result<Zeroizing<String>, &'static str> {
    if !(32..=256).contains(&value.len())
        || !value.is_ascii()
        || value.bytes().any(|byte| byte <= b' ' || byte == 0x7f)
    {
        return Err("relay token must be 32-256 visible ASCII characters");
    }
    Ok(Zeroizing::new(value.to_owned()))
}

pub fn build_relay_path(target: &TelegramTarget) -> Result<String, &'static str> {
    if !matches!(target.dc, 1..=5 | 203) {
        return Err("unsupported Telegram data center");
    }
    Ok(format!(
        "/v1/telegram/dc/{}/{}",
        target.dc,
        if target.media { "media" } else { "main" }
    ))
}

pub fn build_relay_upgrade_request(
    endpoint: &RelayEndpoint,
    target: &TelegramTarget,
    token: &str,
    request_key: &str,
) -> Result<Zeroizing<String>, &'static str> {
    let token = parse_relay_token(token)?;
    if request_key.is_empty()
        || !request_key.is_ascii()
        || request_key.bytes().any(|byte| byte <= b' ' || byte == 0x7f)
    {
        return Err("invalid websocket request key");
    }
    let path = build_relay_path(target)?;
    Ok(Zeroizing::new(format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {request_key}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Protocol: {RELAY_PROTOCOL}\r\n\
         Authorization: Bearer {}\r\n\
         \r\n",
        endpoint.host(),
        token.as_str()
    )))
}
