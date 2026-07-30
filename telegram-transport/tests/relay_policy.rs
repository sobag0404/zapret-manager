use zapret_manager_telegram_transport::{
    relay::{build_relay_path, build_relay_upgrade_request, parse_relay_token, RelayEndpoint},
    TelegramTarget,
};

#[test]
fn relay_endpoint_accepts_only_a_bare_public_wss_origin() {
    let endpoint = RelayEndpoint::parse("wss://user-owned.example").unwrap();
    assert_eq!(endpoint.host(), "user-owned.example");
    assert_eq!(endpoint.port(), 443);

    for rejected in [
        "ws://user-owned.example",
        "https://user-owned.example",
        "wss://user:pass@user-owned.example",
        "wss://user-owned.example/custom",
        "wss://user-owned.example/?dst=1.1.1.1",
        "wss://user-owned.example/#fragment",
        "wss://127.0.0.1",
        "wss://localhost",
    ] {
        assert!(RelayEndpoint::parse(rejected).is_err(), "{rejected}");
    }
}

#[test]
fn relay_path_is_derived_only_from_validated_telegram_target() {
    assert_eq!(
        build_relay_path(&TelegramTarget {
            dc: 2,
            media: false,
        })
        .unwrap(),
        "/v1/telegram/dc/2/main"
    );
    assert_eq!(
        build_relay_path(&TelegramTarget { dc: 4, media: true }).unwrap(),
        "/v1/telegram/dc/4/media"
    );
    assert!(build_relay_path(&TelegramTarget {
        dc: 6,
        media: false,
    })
    .is_err());
}

#[test]
fn relay_upgrade_uses_header_auth_and_never_query_auth() {
    let endpoint = RelayEndpoint::parse("wss://user-owned.example").unwrap();
    let token = parse_relay_token("0123456789abcdef0123456789abcdef").unwrap();
    let request = build_relay_upgrade_request(
        &endpoint,
        &TelegramTarget {
            dc: 2,
            media: false,
        },
        token.as_str(),
        "dGhlIHNhbXBsZSBub25jZQ==",
    )
    .unwrap();
    assert!(request.starts_with("GET /v1/telegram/dc/2/main HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer 0123456789abcdef0123456789abcdef\r\n"));
    assert!(request.contains("Sec-WebSocket-Protocol: zm-telegram-relay-v1\r\n"));
    assert!(!request.contains("?"));
}

#[test]
fn relay_token_is_bounded_and_rejects_header_injection() {
    assert!(parse_relay_token("0123456789abcdef0123456789abcdef").is_ok());
    assert!(parse_relay_token("short").is_err());
    assert!(parse_relay_token(&"a".repeat(257)).is_err());
    assert!(parse_relay_token("0123456789abcdef0123456789abc\r\nX-Injected: yes").is_err());
}
