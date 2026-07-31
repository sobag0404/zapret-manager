# Project Context

Last updated: 2026-07-30

## Current Goal

Zapret Manager v1.2 is a local Windows manager for two experimental modes only:
Discord and YouTube. It is not a VPN, has no account, telemetry, external
server, or system-wide proxy/DNS/firewall configuration.

Telegram, WhatsApp, Tor/WebTunnel, direct Telegram transports, and the
user-owned Cloudflare relay are paused research. They are not exposed in the
GUI, profile manifest, strategy manifest, or release lifecycle. Do not resume
or deploy them without a separate approved scope and a new test-PC proof.

## Protected Artifacts

Never modify, delete, overwrite, or repackage:

- `target/release/bundle/nsis/ZapretManagerSetup.exe`
- `target/release/bundle/nsis/ZapretManager v1.0.exe`
- tag `v1.0`

The current test artifact is always a separately named installer under
`target/release/bundle/nsis/`.

## Latest Test Build

- Product commit `d78704e` was packaged as
  `ZapretManager Discord-YouTube v1.2-test.exe`.
- Installer SHA-256:
  `6C28CEAD5B7FC4D8477BD1A81DC15B55A7D2862537A00D4D53E56883732D14DB`.
- The installer was hash-verified before installation on the Windows 10 test
  PC. Follow-up harness/context commit `1217829` passed CI and Build Windows.

## Confirmed Test-PC Evidence

- Discord-only using existing `alt` (`2 ALT`) started the managed engine and
  returned Discord Web HTTP 200. Earlier smoke also confirmed Discord Desktop
  updater/module initialization and a remote-auth WSS handshake. This is
  test-network evidence only; authenticated chat, media, and voice are not
  claimed.
- YouTube-only using existing `fake_tls_auto` loaded the main page and watch
  page `jNQXAC9IVRw`; previous remote evidence recorded `readyState=4` and
  playback advancing over seven seconds. This is test-network evidence only.
- On 2026-07-30, both Discord-only and YouTube-only engine smokes completed
  cleanup with scoped `winws.exe=0`, app-owned running WinDivert services=0,
  and `engine-runtime` run directories=0.
- The installer built from `d78704e` was installed on the test PC after a
  matching SHA-256 check. Discord-only selected `2 ALT`, started cleanly, and
  returned Discord Web HTTP 200. YouTube-only selected `Fake TLS Auto`; an
  isolated Edge profile loaded a watch page, reached `readyState=4`, advanced
  during playback, completed a seek to 60 seconds, and stayed playable after a
  reload. Both modes were disabled cleanly afterward.
- On the same installed build, closing the main window kept the application
  process alive for the tray. CDP is intentionally torn down with the hidden
  WebView, so tray Exit still needs a separate manual interactive smoke while
  the engine is active.
- Closing the main window intentionally keeps the application in the tray.
  Tray Exit must call the same disable/cleanup path before process exit.

## Product Scope Decision

- Only `discord.json` and `youtube.json` are bundled profiles.
- Only Discord/YouTube strategy entries remain in `strategies/manifest.json`.
- The dashboard selects one mode at a time. Combined Discord+YouTube use is
  fail-closed until separately demonstrated on the test PC.
- Discord selects `2 ALT`; YouTube selects `Fake TLS Auto`. Both remain
  experimental, with test-network wording rather than a global stable claim.
- Other engine command files remain as audited source material only. They are
  not user-selectable and must not be restored to the product without evidence.
- The installer packages an explicit allowlist: the two selected command files,
  their required binaries and shared Discord/YouTube lists. Telegram, WhatsApp,
  Common, and legacy strategy files remain outside the installer resources.

## Safety Invariants

- GUI does not launch arbitrary executables. The backend verifies bundled
  engine files and manifest hashes before launch.
- Only `winws.exe` and WinDivert services scoped under
  `%LOCALAPPDATA%\ZapretManager\engine-runtime` may be stopped or deleted.
- Disable, emergency disable, tray Exit, and forced exit verify managed process
  and driver removal before reporting success. A cleanup failure stays visible
  and retryable.
- Snapshot/revert in v1.2 only covers managed engine/runtime cleanup. It does
  not claim DNS, proxy, firewall, or route restoration because the app does not
  modify those settings.
- No secrets, updater private keys, tokens, cookies, or personal logs may enter
  Git history, diagnostic exports, or documentation.

## Verification Baseline

Run without launching the engine on the main PC:

```powershell
$env:CARGO_BUILD_JOBS='2'; cargo fmt --all --check
$env:CARGO_BUILD_JOBS='2'; cargo test --workspace --locked -j 2
corepack pnpm test
corepack pnpm --dir app/frontend build
cd app/tauri; $env:CARGO_BUILD_JOBS='2'; cargo tauri build
```

Remote tests run only on the designated Windows test PC through the managed
SSH/CDP harness. Record the installed app hash, selected single mode, strategy,
HTTP/UI result, and post-disable scoped cleanup result. Never run the engine on
the main PC.

## Next Release Gate

1. Verify tray Exit and emergency disable interactively with the engine active.
2. Verify an authenticated Discord Desktop session, including media and voice,
   only with explicit user consent; do not store account data in diagnostics.
3. Repeat the installed-build smoke after any engine or lifecycle change.
4. Keep GitHub Actions CI and Build Windows green before promoting a release.
