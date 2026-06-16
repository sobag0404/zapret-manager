Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location app/tauri
try {
  cargo tauri build
} finally {
  Pop-Location
}

$bundle = "target/release/bundle/nsis/Zapret Manager_0.1.0_x64-setup.exe"
$named = "target/release/bundle/nsis/ZapretManagerSetup.exe"
if (Test-Path -LiteralPath $bundle) {
  Copy-Item -LiteralPath $bundle -Destination $named -Force
}
