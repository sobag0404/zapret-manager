Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path engine/manifest.json)) {
  throw "engine/manifest.json is missing"
}
Write-Host "Engine manifest exists. No third-party engine binaries are bundled in v1 scaffold."
