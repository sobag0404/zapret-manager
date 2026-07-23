# Project Context

Last updated: 2026-07-23

## Current Goal

Zapret Manager v1.2: stabilize enable/disable/diagnostics around the local verified engine before trying new DPI strategies. Discord, YouTube, Telegram, and WhatsApp must be treated as unconfirmed until a fresh build is manually tested.

This is a local Windows app. It is not a VPN, does not use a third-party traffic server, does not require an account, and does not collect telemetry.

## Protected Artifacts

Do not modify, delete, overwrite, or repackage:

- `target/release/bundle/nsis/ZapretManagerSetup.exe`
- `target/release/bundle/nsis/ZapretManager v1.0.exe`

Current test installer:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`

Confirmed local install mismatch:

- Installed `C:\Program Files\Zapret Manager\zapret-manager-tauri.exe`: version `1.2.0`, LastWriteTime `2026-06-26 15:29:14`, SHA256 starts `F00F5755`.
- Previous test installer from `2026-06-29`: SHA256 starts `CA67FB58`.
- A local log from `2026-07-08` without `preflight`, `argv_list`, or `build_id` was created by the old installed app, not by the fresh test line.

## Recent Important Commits

- `351c9f7 docs: update project context`
- `916ff52 engine: log launch argv preflight`
- `e03322b diagnostics: prune strategies and add live status`
- `7820515 engine: disable messaging argv injection`
- `cc1b3ef engine: add messaging profile diagnostics`
- `bc0f094 engine: improve telegram whatsapp diagnostics`
- `229b440 cleanup: keep retry state on failure`
- `afaac6d diagnostics: report unconfirmed checks honestly`
- `d27968a profiles: mark modes experimental`
- `bf6dce6 ci: preserve engine resource hashes`
- `3ef41e5 ci: preserve strategy resource hashes`
- `c2399fc ci: rebuild on attributes changes`
- `a30f562 docs: update project context`

## Current Blockers

- User still reports `winws.exe` exits immediately with exit code `1`.
- Latest investigation focuses on launch stability, not choosing a new DPI strategy.
- Old installed build can produce misleading logs; fresh test logs must contain `app_version`, `build_id`, `preflight`, and `argv_list`.
- Strategy status is unknown until validated end-to-end with a live `winws.exe` process and fresh `engine-launch.log`.
- ALT6 is reported broken and must remain hidden/disabled from normal UI/candidates.
- Snapshot/revert for DNS/proxy/firewall is still not implemented; v1.2 only stops the managed engine and cleans runtime state. The app must not claim full DNS/proxy restore.
- General Diagnostics must not claim Windows service, DNS, Internet, Discord, YouTube, Telegram, or WhatsApp are OK without a factual check. Local backend health is separate from Windows service health.

## Current Stabilization Changes

- Frontend startup separates critical state from optional diagnostics/update/log calls so one optional failure does not break the main toggle.
- Build Windows workflow now includes `engine/**`, `profiles/**`, `strategies/**`, and manifest/hash tests.
- Engine manifest hash consistency is tested without running binaries.
- Hashed `engine/local/**` and strategy payloads are stored as exact raw bytes via `.gitattributes`; Git must not convert CRLF/LF and invalidate manifests across operating systems.
- Tauri resources test verifies `engine`, `profiles`, and `strategies` are packaged.
- Launch parser tests cover all visible strategies with a runtime path containing spaces.
- Direct launch now unescapes CMD caret escaping, including `^!`, before building argv.
- Launch logs include build provenance: app version and build id.
- Build id includes `-dirty` when built from uncommitted local changes; final test installer must be rebuilt after the code commit.
- Disable/Exit cleanup keeps enabled state if scoped cleanup fails, so the next action can retry cleanup instead of incorrectly enabling.
- Cleanup failure now reports `RuntimeStatus::Error`; the main toggle shows `Повторить отключение` and calls disable/cleanup again, including orphan-at-start cases.
- Tray Exit closes only after successful scoped cleanup verification; otherwise the app remains open.
- Manual snapshot uses the app data root, not `current_dir()`/Program Files.
- Recovery UI and commands now describe only the safe implemented part: stop managed engine and clean runtime state.
- Diagnostics now marks Windows service checks as skipped, reports local backend separately, and treats DNS/Internet/service availability as unconfirmed until explicit health-checks run.
- User-facing profiles Discord/YouTube/Telegram/WhatsApp/Common are marked `experimental` until manual service access is confirmed.
- Remote testing harness is being added for a separate Windows 10 PC over SSH/Tailscale. It launches the installed GUI with WebView2 CDP on loopback only and does not start the engine by itself.

## Verified In Current Block

Passed locally so far:

- `cargo fmt --all --check`
- `cargo test --workspace`
- `corepack pnpm test`
- `corepack pnpm --dir app/frontend build`
- `cargo tauri build`
- Fresh `target/release/bundle/nsis/ZapretManager v1.2-test.exe` was rebuilt after the latest code commit.
- Protected `ZapretManagerSetup.exe` and `ZapretManager v1.0.exe` were checked unchanged.

GitHub Actions:

- `c2399fc`: CI #43 passed.
- `c2399fc`: Build Windows #33 passed and uploaded the Windows artifact.

## Manual Test Instructions After Fresh Build

Install the new `ZapretManager v1.2-test.exe` over the old Program Files build. Do not use logs from the old installed app.

After pressing Enable, if it fails, export diagnostics and send:

- the new `engine-launch.log`;
- `diagnostic-export.txt`;
- the visible build id shown in Diagnostics.

Fresh logs must include `app_version`, `build_id`, `preflight_ok`, `preflight_report`, and `argv_list`.

## Remote Test Harness

Use `docs/REMOTE_TESTING.md` and scripts under `tools/remote-test/` for reproducible tests on a separate Windows 10 PC. CDP must stay bound to `127.0.0.1` and be accessed through an SSH tunnel. The coordinator performs real engine/access tests on the remote PC; this repo only provides safe tooling and diagnostics.

## Security Rules

- Never commit secrets, GitHub tokens, `.env`, updater private keys, cookies, or private logs.
- `.tauri-updater/` and signing keys stay local/secret only.
- Do not add or replace engine binaries without trusted source review and `engine/manifest.json` hash updates.
- Do not run third-party scripts or binaries unless reviewed and required.
- Do not log user traffic, private messages, cookies, tokens, passwords, or personal data.
- External engine files are untrusted until manifest/source/hash verification passes.

## Backlog

- Automatic strategy selection by profile health-check after lifecycle stabilization.
- Start with manual `Следующая стратегия` and later `Подобрать автоматически`.
- Health-checks only use DNS resolve, TCP connect, and HTTPS connect.
- No user traffic inspection.
- No infinite switching; use cooldown and attempt limits.
