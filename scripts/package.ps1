Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$dirty = & git -C $repoRoot status --porcelain
if ($LASTEXITCODE -ne 0) {
  throw "Unable to determine Git state."
}
if ($dirty) {
  throw "Refusing to package a dirty worktree. Commit or stash changes first."
}

$buildId = (& git -C $repoRoot rev-parse --short=12 HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $buildId -notmatch '^[0-9a-fA-F]{12}$') {
  throw "Unable to determine a valid Git build id."
}
$env:ZAPRET_MANAGER_BUILD_ID = $buildId.ToLowerInvariant()

Push-Location app/tauri
try {
  cargo tauri build
} finally {
  Pop-Location
}

$version = "1.3.1"
$bundle = "target/release/bundle/nsis/Zapret Manager_1.3.1_x64-setup.exe"
$named = "target/release/bundle/nsis/ZapretManager Discord-YouTube v$version.exe"
if (Test-Path -LiteralPath $bundle) {
  Copy-Item -LiteralPath $bundle -Destination $named -Force
  Write-Host "Created $named"
}

$baseline = "target/release/bundle/nsis/ZapretManagerSetup.exe"
if (Test-Path -LiteralPath $baseline) {
  Write-Host "Baseline preserved: $baseline"
}
