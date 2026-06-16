Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

corepack pnpm test -- tests/strategy_validation.test.ts tests/manifest_validation.test.ts
