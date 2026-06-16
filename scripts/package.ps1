Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

corepack pnpm build
cargo tauri build --manifest-path app/tauri/Cargo.toml
