Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location app/tauri
try {
  cargo tauri build
} finally {
  Pop-Location
}

$version = "1.2"
$bundle = "target/release/bundle/nsis/Zapret Manager_1.2.0_x64-setup.exe"
$named = "target/release/bundle/nsis/ZapretManager v$version-test.exe"
if (Test-Path -LiteralPath $bundle) {
  Copy-Item -LiteralPath $bundle -Destination $named -Force
  Write-Host "Created $named"
}

$baseline = "target/release/bundle/nsis/ZapretManagerSetup.exe"
if (Test-Path -LiteralPath $baseline) {
  Write-Host "Baseline preserved: $baseline"
}
