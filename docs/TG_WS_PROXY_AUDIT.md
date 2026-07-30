# TG WS Proxy Audit

Date: 2026-07-30

## Decision

Do not integrate Flowseal `tg-ws-proxy` into Zapret Manager as an executable,
embedded dependency, or automatic download in v1.2. The audited upstream source
is MIT licensed, but its current security and routing model does not meet this
project's local-only, manifest-verified, and no-untrusted-update requirements.

## Reviewed Source

- Repository: `https://github.com/Flowseal/tg-ws-proxy`
- Reviewed commit: `21aaeb3aba97ad3b0ae39c6540a7b1afd12a3f7e`
- License: MIT
- No upstream executable was downloaded, executed, packaged, or added to this
  repository.

## Blocking Findings

1. The default configuration enables CF proxy fallback and periodic retrieval of
   a domain list from the mutable `main` branch on GitHub. The fetched list is
   not pinned to an immutable commit, signed manifest, or hash.
2. Fallback routes can carry Telegram traffic through external CF proxy or
   worker endpoints. This conflicts with Zapret Manager's stated local-only
   traffic model unless the user explicitly accepts a separately designed routed
   mode and its privacy consequences.
3. The upstream Windows application implements self-update by downloading and
   replacing an executable without a project-owned signature, pinned manifest,
   or hash verification gate compatible with Zapret Manager.
4. The Python build has direct dependency versions but no full hash-locked
   transitive dependency set or reproducible release attestation. The release
   workflow builds a PyInstaller executable but does not publish a signed
   provenance/manifest contract that this app can verify.
5. The proxy terminates and re-encrypts Telegram transport bytes locally. It is
   therefore a sensitive data-plane component and cannot be treated as a simple
   opaque helper process. Its logs, secret generation, restart behavior, and
   cleanup lifecycle would need a dedicated review and a managed API boundary.
6. The upstream tray application owns its own process, settings, update, and
   restart behavior. Zapret Manager cannot currently guarantee its required
   Disable, tray Exit, crash-recovery, and uninstall cleanup invariants for that
   external lifecycle.

## Required Preconditions For Reconsideration

Any future Telegram Desktop routed mode must be a separate opt-in feature with:

- a project-owned source build or separately audited component;
- immutable source/version pinning and a signed manifest with per-file hashes;
- no automatic updates and no mutable remote domain lists;
- no external fallback endpoints unless the UI explicitly explains the third
  party route and obtains consent;
- a loopback-only listener on a reserved app-owned port, random per-install
  secret, and redacted logs;
- a supported Telegram Desktop consent flow that does not edit private client
  files silently;
- process, listener, proxy-setting, and temporary-data verification on Disable,
  tray Exit, crash recovery, and uninstall;
- a clean test-PC proof before any user-facing availability claim.

## Test-PC Blocker

SSH connectivity to the test PC was restored on 2026-07-30 after restarting
Tailscale on the main PC.

A project-owned Rust proof of concept was then built from the minimum reviewed
transport logic. It does not embed or run the upstream application and removes
CF/external fallback, mutable lists, updater, tray, telemetry-like statistics,
and arbitrary destination input. It adds certificate verification, a strict
loopback listener, official Telegram domain/CIDR allowlists, secret redaction,
bounded sessions, and locked Rust dependencies.

The proof did not pass the remote product gate. TLS WebSocket probes to both
official DC2 hostnames and the official DC4 pair timed out before a WebSocket
upgrade. Independent TCP 443 checks for the two DC2 hostnames also timed out.
Because no official WebSocket transport could be reached, Telegram Desktop was
not configured and no user traffic was passed through the component.

The proof remains compile/test-only research and is not packaged by Tauri,
exposed in the UI, or started by Zapret Manager. Integration is blocked until a
test network can reach an official Telegram WebSocket endpoint without a
third-party relay. See `docs/TELEGRAM_DESKTOP_TRANSPORT_SECURITY.md`.
