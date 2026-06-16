param(
  [string]$InstallDir = "$env:ProgramFiles\ZapretManager"
)

$serviceExe = Join-Path $InstallDir "ZapretManagerService.exe"
if (Test-Path -LiteralPath $serviceExe) {
  & $serviceExe emergency-disable
}

Write-Host "Mock uninstall service command completed."
