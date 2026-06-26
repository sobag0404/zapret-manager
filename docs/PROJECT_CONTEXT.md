# Project Context

Last updated: 2026-06-26

## Current Goal

Zapret Manager v1.2: a simple local Windows desktop app that can enable/disable selected profiles, run a verified local engine, cleanly stop on Disable/Exit, and provide enough logs to debug failed `winws.exe` starts.

This is a local Windows application. It is not a VPN, does not use a third-party traffic server, does not require an account, and does not collect telemetry.

## Protected Artifacts

Do not modify, delete, overwrite, or repackage:

- `target/release/bundle/nsis/ZapretManagerSetup.exe`
- `target/release/bundle/nsis/ZapretManager v1.0.exe`

Current test installer:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`

## Recent Important Commits

- `62c15aa ci: use node tauri cli`
- `1b8a843 engine: cleanup runtime lifecycle`
- `28f8de3 ci: separate windows build`
- `863e0c9 ci: fix actions setup`
- `479a74c engine: fix launch diagnostics`
- `e12ab2a engine: launch winws directly`
- `a3a3247 engine: skip service bat launcher`
- `12147de updater: v1.2 safe updates`

## Done

- Tauri/React UI with profiles for Discord, YouTube, Telegram, WhatsApp, and Common.
- Real Flowseal-style engine bundle is launched through a checked runtime copy, not directly from GUI resources.
- Direct `winws.exe` launch path is implemented from selected strategy `.bat`.
- Enable now creates snapshot state before launch.
- Disable/emergency disable/tray Exit attempt cleanup and reset runtime state.
- `engine-launch.log` includes strategy, admin state, work dir, `winws.exe`, WinDivert file presence, argv count, and command.
- Early `winws.exe` exit is handled as failure and does not leave the app marked as running.
- CI split:
  - Ubuntu checks cross-platform Rust crates and frontend.
  - Windows checks full Rust workspace.
  - Windows build produces installer artifact.
- Node Tauri CLI is used in Actions to avoid slow `cargo install tauri-cli`.

## Current Problems / Blockers

- User still needs to test the latest `ZapretManager v1.2-test.exe` after runtime cleanup changes.
- If enable fails again, the next required input is the new `engine-launch.log` path shown by the app.
- Telegram/WhatsApp strategy effectiveness is not confirmed stable yet.
- Snapshot/revert is still mostly architectural/mock for system DNS/proxy/firewall state.

## Verified

Local checks passed on 2026-06-26:

- `cargo fmt --all --check`
- `cargo test --workspace`
- `corepack pnpm test`
- `corepack pnpm --dir app/frontend build`
- Tauri installer build from `app/tauri`
- Test installer copied to `target/release/bundle/nsis/ZapretManager v1.2-test.exe`

GitHub Actions on latest pushed commit:

- CI: success
- Build Windows: success

## Remaining Before Stable v1.2

- User manual test on latest test installer.
- Confirm Disable, Emergency Disable, tray Exit, and app shutdown leave no `winws.exe` process.
- Confirm VPN does not complain after full tray Exit.
- Confirm `engine-launch.log` gives actionable detail if `winws.exe` exits immediately.
- Decide whether Telegram/WhatsApp need profile-specific strategy changes.
- Create signed release only after user confirms the test build works.

## Security Rules

- Never commit secrets, GitHub tokens, `.env`, updater private keys, cookies, or private logs.
- `.tauri-updater/` and signing keys stay local/secret only.
- Do not add or replace engine binaries without trusted source review and `engine/manifest.json` hash updates.
- Do not run third-party scripts or binaries unless they are reviewed and required for the task.
- Do not log user traffic, private messages, cookies, tokens, passwords, or personal data.
- External engine files are untrusted until manifest/source/hash verification passes.

## Backlog

Future feature after lifecycle stabilization:

- Automatic strategy selection by profile health-check.
- Start with manual `Следующая стратегия` and `Подобрать автоматически`.
- Health-checks only use DNS resolve, TCP connect, and HTTPS connect.
- No user traffic inspection.
- No infinite switching; use cooldown and attempt limits.
- Do not break working profiles while trying to fix another profile.
