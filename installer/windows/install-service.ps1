param(
  [string]$InstallDir = "$env:ProgramFiles\ZapretManager"
)

$serviceExe = Join-Path $InstallDir "ZapretManagerService.exe"
if (-not (Test-Path -LiteralPath $serviceExe)) {
  throw "Service executable not found: $serviceExe"
}

Write-Host "Mock install service command. Real service registration is implemented in the service binary in the next stage."
