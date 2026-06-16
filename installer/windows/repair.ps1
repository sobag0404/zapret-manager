param(
  [string]$InstallDir = "$env:ProgramFiles\ZapretManager"
)

$serviceExe = Join-Path $InstallDir "ZapretManagerService.exe"
if (Test-Path -LiteralPath $serviceExe) {
  & $serviceExe diagnostics
}

Write-Host "Mock repair completed."
