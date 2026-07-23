param(
  [string]$AppPath = "C:\Program Files\Zapret Manager\zapret-manager-tauri.exe",
  [ValidateRange(1024, 65535)]
  [int]$CdpPort = 9223,
  [ValidateSet("ScheduledTask", "Direct")]
  [string]$LaunchMode = "ScheduledTask",
  [string]$StateDir = (Join-Path $env:LOCALAPPDATA "ZapretManager\remote-test"),
  [switch]$ValidateOnly
)

$ErrorActionPreference = "Stop"

function Escape-SingleQuotedPowerShell([string]$Value) {
  return $Value.Replace("'", "''")
}

function Test-CdpReady([int]$Port) {
  try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/version" -TimeoutSec 2
    return $null -ne $response
  } catch {
    return $false
  }
}

function Assert-LoopbackOnly([int]$Port) {
  $listeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
  foreach ($listener in $listeners) {
    if ($listener.LocalAddress -ne "127.0.0.1" -and $listener.LocalAddress -ne "::1") {
      throw "CDP port $Port is listening on $($listener.LocalAddress), expected loopback only."
    }
  }
}

function Get-InteractiveTaskUserId {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  if (-not $identity -or [string]::IsNullOrWhiteSpace($identity.Name)) {
    throw "Unable to resolve current Windows identity for scheduled task principal."
  }
  return $identity.Name
}

if (-not (Test-Path -LiteralPath $AppPath -PathType Leaf)) {
  throw "Zapret Manager executable not found: $AppPath"
}

$existingListener = Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue
if ($existingListener) {
  Assert-LoopbackOnly -Port $CdpPort
  throw "Port $CdpPort is already in use. Pick another local CDP port."
}

New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
$runId = "zm-remote-test-" + (Get-Date -Format "yyyyMMdd-HHmmss")
$userDataFolder = Join-Path $StateDir "webview2-user-data-$CdpPort"
$runnerPath = Join-Path $StateDir "launch-$runId.ps1"
$statePath = Join-Path $StateDir "state-$runId.json"
$workDir = Split-Path -Parent $AppPath
$taskName = $null

if ($ValidateOnly) {
  [pscustomobject]@{
    ok = $true
    app_path = $AppPath
    cdp_endpoint = "http://127.0.0.1:$CdpPort"
    launch_mode = $LaunchMode
    state_dir = $StateDir
    note = "Validation only; Zapret Manager was not started."
  } | ConvertTo-Json -Depth 4
  exit 0
}

$runner = @"
`$ErrorActionPreference = "Stop"
`$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$CdpPort"
`$env:WEBVIEW2_USER_DATA_FOLDER = '$(Escape-SingleQuotedPowerShell $userDataFolder)'
`$env:ZAPRET_MANAGER_REMOTE_TEST_ID = '$runId'
`$env:ZAPRET_MANAGER_REMOTE_TEST_CDP_PORT = '$CdpPort'
`$process = Start-Process -FilePath '$(Escape-SingleQuotedPowerShell $AppPath)' -WorkingDirectory '$(Escape-SingleQuotedPowerShell $workDir)' -PassThru
`$state = [pscustomobject]@{
  run_id = '$runId'
  started_at = (Get-Date).ToString("o")
  app_path = '$(Escape-SingleQuotedPowerShell $AppPath)'
  app_pid = `$process.Id
  cdp_endpoint = 'http://127.0.0.1:$CdpPort'
  remote_test_cdp_port = '$CdpPort'
  webview2_user_data_folder = '$(Escape-SingleQuotedPowerShell $userDataFolder)'
  state_path = '$(Escape-SingleQuotedPowerShell $statePath)'
}
`$state | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath '$(Escape-SingleQuotedPowerShell $statePath)' -Encoding UTF8
"@

Set-Content -LiteralPath $runnerPath -Value $runner -Encoding UTF8

if ($LaunchMode -eq "Direct") {
  & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runnerPath
} else {
  $taskName = "ZapretManagerRemoteTest-$runId"
  $userId = Get-InteractiveTaskUserId
  $action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$runnerPath`""
  $trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1)
  $principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType Interactive -RunLevel Highest
  Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Force | Out-Null
  Start-ScheduledTask -TaskName $taskName
}

$deadline = (Get-Date).AddSeconds(60)
while ((Get-Date) -lt $deadline -and -not (Test-Path -LiteralPath $statePath)) {
  Start-Sleep -Milliseconds 500
}
if (-not (Test-Path -LiteralPath $statePath)) {
  throw "Zapret Manager launch state was not written. Check that an interactive Windows session is logged on."
}

if ($taskName) {
  Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
}

$deadline = (Get-Date).AddSeconds(60)
while ((Get-Date) -lt $deadline) {
  if (Test-CdpReady -Port $CdpPort) {
    Assert-LoopbackOnly -Port $CdpPort
    $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $state | Add-Member -NotePropertyName ready -NotePropertyValue $true -Force
    $state | Add-Member -NotePropertyName ssh_tunnel -NotePropertyValue "ssh -L $CdpPort`:127.0.0.1:$CdpPort <windows-test-user>@<tailscale-host>" -Force
    $state | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $statePath -Encoding UTF8
    $state | ConvertTo-Json -Depth 4
    exit 0
  }
  Start-Sleep -Seconds 1
}

throw "Zapret Manager started, but WebView2 CDP did not become ready on 127.0.0.1:$CdpPort."
