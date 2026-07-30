# Telegram Desktop Transport Security Decision

Date: 2026-07-30

Status: proof of concept only; production integration blocked

## Decision

Zapret Manager does not integrate or package the Telegram Desktop transport.
An isolated Rust proof of concept was implemented to test whether a direct,
official Telegram WebSocket route is feasible on the Windows 10 test network.
The network gate failed before Telegram Desktop configuration, so no GUI,
installer, updater, or application lifecycle code was changed.

## Provenance

- Reviewed upstream: `Flowseal/tg-ws-proxy`
- Immutable source revision:
  `21aaeb3aba97ad3b0ae39c6540a7b1afd12a3f7e`
- Upstream license: MIT; preserved under
  `third_party/flowseal-tg-ws-proxy/LICENSE`
- Rust dependency versions and registry checksums: `Cargo.lock`
- CI and release tests use `cargo --locked`
- Official Telegram network ranges:
  `https://core.telegram.org/resources/cidr.txt`, reviewed 2026-07-30

No upstream executable, release archive, updater, mutable domain list, CF
worker, CF proxy, public proxy list, telemetry, or independent tray component
is downloaded, executed, or packaged.

## Bounded Architecture

```text
Telegram Desktop
  -> 127.0.0.1:<ephemeral port>
  -> project-owned MTProto transport re-encryption
  -> certificate-verified wss://kws<dc>[-1].web.telegram.org/apiws
  -> official Telegram network ranges only
```

Controls implemented in the research crate:

- literal IPv4 loopback bind only;
- random 16-byte secret supplied through an app-owned file;
- no secret, client address, packet payload, message, or media logging;
- DC allowlist `1..5` plus Telegram's special DC `203`;
- generated official hostname allowlist only;
- TLS certificate and hostname verification;
- resolved address must match an official Telegram CIDR;
- WebSocket `101`, Upgrade headers, and `Sec-WebSocket-Accept` validation;
- masked client frames, frame and buffer bounds, eight-session limit;
- short pre-auth timeout and bounded idle timeout;
- tracked task cancellation and drain on shutdown;
- upstream known-answer vectors for obfuscated-handshake and AES-CTR bridge
  parity;
- no `unsafe` Rust.

## Threat Model

| Threat | Control | Residual risk |
| --- | --- | --- |
| Malicious or mutable upstream | Immutable revision and local reimplementation | Future upstream protocol changes require a new audit |
| Arbitrary traffic proxy | Loopback bind, Telegram-only domains and CIDRs | A compromised Telegram endpoint remains in the trust boundary |
| DNS rebinding | Resolve once, connect to validated Telegram CIDR, verify TLS hostname | Official DNS/PKI compromise is out of scope |
| Secret disclosure | Local random secret, zeroizing containers, redacted status/logs | Same-user memory inspection is out of scope |
| Traffic-content leakage | No payload logging; component lacks Telegram account auth keys | The component handles transport bytes in memory |
| Local denial of service | Session cap, handshake timeout, frame/buffer bounds | A same-user process can still cause bounded connection churn |
| Orphan process/listener | Tracked session cancellation; product integration would require scoped process verification | Product lifecycle is not implemented because the network gate failed |
| Supply-chain substitution | Cargo registry checksums, locked CI, preserved attribution | Release signing/SBOM are required before any future packaging |

## Remote Evidence

The tested research executable had SHA-256
`1DAAFAB5C7C0FD7648E97593518D451F870415313B4929C991444A9BADFA8186`.
It was transferred to the isolated Windows 10 test PC and the hash matched.

Three direct official-path attempts were bounded:

1. primary DC2 WebSocket hostname;
2. alternate DC2 WebSocket hostname;
3. the official DC4 hostname pair.

All timed out before WebSocket upgrade. Independent TCP 443 checks for both
DC2 hostnames also failed. No external fallback was enabled or attempted.

After the test:

- transport process count: zero;
- `winws.exe` count: zero;
- running app-owned WinDivert count: zero;
- PoC staging directory: absent;
- Windows global proxy: disabled, no proxy server, no PAC URL.

Telegram Desktop proxy configuration was never changed because the upstream
connectivity gate did not pass.

## Production Gate

Production work may resume only after all of the following are true:

1. An isolated test PC reaches an official Telegram WebSocket endpoint.
2. Telegram Desktop connects through a user-consented local proxy.
3. Messages, media, reconnect, and restart are manually confirmed without
   logging content.
4. A supported method restores Telegram Desktop's previous proxy state without
   editing undocumented private files.
5. Disable, tray Exit, crash recovery, and uninstall remove the process,
   listener, secret file, and app-owned runtime.
6. An independent security review approves the final lifecycle and release
   package.

On the current test network, a direct official-only transport is not feasible.
A working solution requires a changed route such as the user's VPN or a
separately consented external proxy. That would change the product's local-only
privacy statement and is not silently substituted.
