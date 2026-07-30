# Telegram Cloudflare Relay Security Design

Status: design and deploy-gated proof only. This mode is not packaged or
exposed in the Zapret Manager release until a test-PC Telegram Desktop smoke
passes.

## Decision

The direct Telegram WebSocket route is blocked on the current test network.
An optional Telegram Desktop mode may therefore use a Cloudflare Worker owned
and deployed by the user. It is a separate routed mode and does not replace or
modify the local Discord and YouTube modes.

The UI must state before consent:

- Telegram traffic uses the user's external Cloudflare Worker.
- Cloudflare can observe connection metadata and carries an encrypted MTProto
  byte stream.
- Zapret Manager does not provide anonymity and cannot control Cloudflare's
  infrastructure or account security.
- The feature is disabled by default and requires a user-supplied Worker URL
  and relay secret.

## Data Flow And Trust Boundaries

```text
Telegram Desktop
  -> MTProto proxy on 127.0.0.1
  -> Zapret Manager transport
  -> authenticated WSS/TLS
  -> user's Cloudflare Worker
  -> raw TCP 443
  -> hardcoded official Telegram DC IP
```

Trust boundaries:

1. Telegram Desktop to the app-owned loopback listener.
2. The local process to Cloudflare over certificate-verified WSS/TLS.
3. Cloudflare's Worker runtime to an allowlisted Telegram DC over raw TCP 443.
4. Local configuration storage containing the endpoint and relay secret.

TLS is mandatory on the Internet-facing local-to-Worker leg. Telegram's DC
endpoint speaks MTProto rather than TLS, so the Worker-to-Telegram leg is raw
TCP carrying the already encrypted MTProto stream. The UI must not claim that
this second leg is TLS.

Cloudflare is an external processor in this mode. It can observe source,
destination, timing, volume, and connection duration. The Worker must not log
payloads, authorization values, client IP addresses, or destination metadata.
MTProto payload bytes remain encrypted end to end by Telegram; the Worker
forwards opaque bytes and has no Telegram account keys.

## Security Invariants

- Worker ingress is WebSocket over HTTPS only.
- The request path selects only a compiled-in DC and main/media role.
- The request cannot supply an IP, hostname, port, redirect, or arbitrary URL.
- Outbound targets are immutable official Telegram IPs on TCP port 443.
- Authorization is an exact secret comparison; the secret is stored as a
  Cloudflare Worker secret and in an app-owned local file.
- No KV, D1, R2, analytics, telemetry, mutable remote configuration, updater,
  public proxy list, or HTTP destination is used.
- Worker messages, queues, sessions, connect attempts, and idle duration are
  bounded.
- Worker-to-client delivery uses a typed sequence/ack envelope and allows only
  one unacknowledged 64 KiB payload, preventing an unbounded WebSocket send
  queue. Relay-mode uploads are split to the same bound without changing the
  byte stream.
- Per-isolate admission is limited to four sessions, with at most 2 MiB of
  queued ingress per session. This is a memory bound, not an account-wide
  quota.
- TCP write timeout cleanup closes the socket before aborting the writer, and
  connection leases are released idempotently.
- The local endpoint parser accepts only a bare `wss://` origin with no
  credentials, query, fragment, custom path, localhost, or IP literal.
- Logs and status contain only generic state and redact both the endpoint and
  relay secret.
- Disable, tray Exit, crash recovery, and uninstall must remove the local
  process, loopback listener, secret/runtime files, and any Telegram proxy
  setting before this mode can enter a release.

## Threats And Controls

| Threat | Control | Residual risk |
| --- | --- | --- |
| Open proxy or SSRF | No client destination parameter; closed route table; port fixed to 443 | A compromised release can change the table |
| Unauthorized relay use | User-owned high-entropy Worker secret over TLS; generic rejection | Per-isolate limits are not an account-wide quota |
| DNS rebinding or redirect | Worker connects to IP literals only; local relay origin allows no redirect | Cloudflare and Telegram routing remain trusted |
| Secret disclosure | Wrangler secret binding; app-owned local file; no query token; redaction | Same-user memory inspection is out of scope |
| Payload or identity logging | No logging calls or analytics bindings in Worker | Cloudflare platform metadata remains visible |
| Memory/CPU denial of service | Bounded frames, queue, rate, session, connect and idle timeouts | The connection cap is per isolate, not account-wide; distributed authenticated abuse can consume the user's free quota |
| Stale proxy after failure | Product gate requires verified lifecycle restoration | Not satisfied until product integration and remote smoke |
| Supply-chain substitution | Pinned npm versions, frozen lockfile, CI tests, source in repository | Cloudflare runtime is an external dependency |

## Deployment Gate

1. `wrangler whoami` confirms the user's account without printing credentials.
2. Select the intended account explicitly and a unique test Worker name.
   Deployment tooling must refuse an existing name; do not rely on cached
   Wrangler account state or a fixed deploy script.
3. Deploy a separately named test Worker; never overwrite another Worker.
4. Store the relay token with `wrangler secret put RELAY_TOKEN`; never commit
   it or pass it in a URL.
5. Verify non-WebSocket, unauthenticated, query-destination, unknown-DC, and
   oversized-message requests fail closed.
6. On the test PC only, verify Telegram Desktop messages, media, restart, and
   proxy restoration.
7. Verify transport process, loopback listener, runtime files, and proxy
   setting are absent after Disable and full Exit.

Until every gate passes, the source remains an auditable PoC and must not be
included in the production Tauri resources or user-facing release UI.
