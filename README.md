# Zapret Manager

Zapret Manager is a local Windows desktop application for managing safe,
reversible `zapret` engine profiles through a GUI and a local Windows service.

It is not a VPN, does not use an external server, does not route user traffic
through third-party servers, and does not collect telemetry. The initial version
ships with a mock engine adapter and safe strategy/profile scaffolding only.

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

## CI

The GitHub Actions workflows are under `.github/workflows/`:

- `ci.yml` runs frontend install/test/build and Rust workspace tests.
- `build-windows.yml` builds realistic Windows artifacts from the frontend and
  Rust workspace.
- `release.yml` is a guarded release skeleton for tagged builds and artifact
  publishing.

## Safety Principles

- Do not apply engine updates without a policy decision and a rollback path.
- Do not write irreversible service, network, or firewall changes.
- Do not collect packet contents, credentials, tokens, or unrelated browsing
  data in diagnostics.
- Prefer explicit user consent for privileged operations.

## License

MIT. See [LICENSE](LICENSE).
