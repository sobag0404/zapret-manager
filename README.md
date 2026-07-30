# Zapret Manager

Zapret Manager is a local Windows desktop application for managing verified
`zapret`/`winws` engine profiles through a GUI.

It is not a VPN, does not use an external server, does not route user traffic
through third-party servers, and does not collect telemetry. The Windows build
can bundle a verified Flowseal `zapret-discord-youtube` engine package and
starts it only after manifest and SHA-256 checks pass.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Installation](docs/INSTALLATION.md)
- [Security and threat model](docs/SECURITY.md)
- [Engine policy](docs/ENGINE_POLICY.md)
- [Update policy](docs/UPDATE_POLICY.md)
- [Revert policy](docs/REVERT_POLICY.md)
- [Diagnostics](docs/DIAGNOSTICS.md)
- [Recovery](docs/RECOVERY.md)
- [Windows service](docs/WINDOWS_SERVICE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

## Development

Prerequisites:

- Windows 10/11 for service and installer work.
- Rust stable toolchain.
- Node.js with Corepack enabled.
- pnpm for frontend tasks.

Common checks:

```powershell
corepack enable
pnpm install
pnpm test
pnpm build
cargo test --workspace --all-features
```

Development GUI:

```powershell
corepack pnpm --dir app/frontend dev
cd app/tauri
cargo tauri dev
```

Build Windows installer:

```powershell
corepack pnpm install
.\scripts\package.ps1
```

The installer artifact is produced by Tauri/NSIS under
`target/release/bundle/nsis/`. The packaging script refuses a dirty worktree
and embeds the checked-out Git commit in the build identity.

The current v1.2 product scope contains two experimental modes only: Discord
and YouTube. When the user presses `Включить`, the app selects the verified
test-network candidate for the selected single mode and starts `winws.exe`
through Windows UAC (`runas`). Closing the window hides the app to tray;
choosing `Выход` in the tray stops the engine first.

Available product strategies:

- Discord: `2 ALT` (experimental; test-network evidence for Web/Desktop)
- YouTube: `Fake TLS Auto` (experimental; test-network evidence for Web/video)

Discord and YouTube are started separately until a combined-mode test is
confirmed. Telegram, WhatsApp, and routed-transport research are intentionally
not part of this installer.

## CI

The GitHub Actions workflows are under `.github/workflows/`:

- `ci.yml` runs frontend install/test/build and Rust workspace tests.
- `build-windows.yml` builds realistic Windows artifacts from the frontend and
  Rust workspace.
- `release.yml` is a guarded release skeleton for tagged builds and artifact
  publishing.

## Safety Principles

- Do not apply engine updates without a policy decision and a rollback path.
- Do not write irreversible service, network, DNS, proxy, or firewall changes.
- Do not collect packet contents, credentials, tokens, or unrelated browsing
  data in diagnostics.
- Prefer explicit user consent for privileged operations.

## License

MIT. See [LICENSE](LICENSE).
