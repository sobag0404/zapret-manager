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
corepack pnpm build
cd app/tauri
cargo tauri build
```

The installer artifact is produced by Tauri/NSIS under
`target/release/bundle/nsis/`.

The GUI starts normally. When the user presses `Включить`, the app launches
`winws.exe` with Windows UAC (`runas`) because WinDivert cannot start without
elevated rights. Closing the window hides the app to tray; choosing `Закрыть`
in the tray menu stops the engine first.

Available engine strategies in the UI:

- `General`
- `ALT`
- `ALT2`
- `ALT3`
- `Simple Fake`
- `Fake TLS Auto`

If one strategy does not work for the current provider, turn the mode off,
select another strategy on the main screen, and turn it on again.

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
