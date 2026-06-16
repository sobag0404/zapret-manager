Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

corepack pnpm install
corepack pnpm build
cargo build --workspace
