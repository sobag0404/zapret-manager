use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use zeroize::Zeroizing;

pub mod protocol;
pub mod relay;
pub mod server;
mod websocket;

pub const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramTarget {
    pub dc: u16,
    pub media: bool,
}

pub fn validate_listener(address: SocketAddr) -> Result<(), &'static str> {
    if address.ip() != LOOPBACK {
        return Err("listener must bind to 127.0.0.1");
    }
    Ok(())
}

pub fn official_ws_domains(target: &TelegramTarget) -> Result<[String; 2], &'static str> {
    let dc = match target.dc {
        1..=5 => target.dc,
        203 => 2,
        _ => return Err("unsupported Telegram data center"),
    };
    let primary = format!("kws{dc}.web.telegram.org");
    let media = format!("kws{dc}-1.web.telegram.org");
    Ok(if target.media {
        [media, primary]
    } else {
        [primary, media]
    })
}

pub fn redact_log(message: &str, secret: &[u8; 16]) -> String {
    let raw = String::from_utf8_lossy(secret);
    message
        .replace(&hex::encode(secret), "[REDACTED]")
        .replace(&hex::encode_upper(secret), "[REDACTED]")
        .replace(raw.as_ref(), "[REDACTED]")
}

pub fn parse_secret(value: &str) -> Result<[u8; 16], &'static str> {
    let decoded = Zeroizing::new(hex::decode(value).map_err(|_| "secret must be hexadecimal")?);
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| "secret must be exactly 16 bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_is_loopback_only() {
        assert!(validate_listener("127.0.0.1:1443".parse().unwrap()).is_ok());
        assert!(validate_listener("0.0.0.0:1443".parse().unwrap()).is_err());
        assert!(validate_listener("[::]:1443".parse().unwrap()).is_err());
        assert!(validate_listener("[::1]:1443".parse().unwrap()).is_err());
    }

    #[test]
    fn upstream_domains_are_closed_over_known_telegram_dcs() {
        assert_eq!(
            official_ws_domains(&TelegramTarget {
                dc: 2,
                media: false,
            })
            .unwrap(),
            [
                "kws2.web.telegram.org".to_string(),
                "kws2-1.web.telegram.org".to_string(),
            ]
        );
        assert!(official_ws_domains(&TelegramTarget {
            dc: 6,
            media: false,
        })
        .is_err());
        assert!(official_ws_domains(&TelegramTarget {
            dc: 0,
            media: false,
        })
        .is_err());
    }

    #[test]
    fn media_prefers_the_dash_one_official_endpoint() {
        assert_eq!(
            official_ws_domains(&TelegramTarget { dc: 4, media: true }).unwrap(),
            [
                "kws4-1.web.telegram.org".to_string(),
                "kws4.web.telegram.org".to_string(),
            ]
        );
    }

    #[test]
    fn logs_redact_raw_and_hex_secret() {
        let secret = [0xabu8; 16];
        let raw = String::from_utf8_lossy(&secret);
        let hex = "abababababababababababababababab";
        let input = format!("secret={hex}; bytes={raw}");
        let redacted = redact_log(&input, &secret);
        assert!(!redacted.contains(hex));
        assert!(!redacted.contains(raw.as_ref()));
        assert_eq!(redacted, "secret=[REDACTED]; bytes=[REDACTED]");
    }

    #[test]
    fn secret_parser_requires_exact_lower_or_upper_hex() {
        assert_eq!(
            parse_secret("00112233445566778899aAbBcCdDeEfF").unwrap(),
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ]
        );
        assert!(parse_secret("0011").is_err());
        assert!(parse_secret("00112233445566778899aabbccddeezz").is_err());
    }
}
