param(
  [string]$StateDir = (Join-Path $env:LOCALAPPDATA "ZapretManager\remote-test"),
  [string]$RunId,
  [string]$DataRoot = (Join-Path $env:LOCALAPPDATA "ZapretManager"),
  [switch]$CleanupScopedWinws
)

$ErrorActionPreference = "Stop"

function Get-StateFile {
  if ($RunId) {
    $path = Join-Path $StateDir "state-$RunId.json"
    if (-not (Test-Path -LiteralPath $path)) {
      throw "State file not found: $path"
    }
    return $path
  }
  $latest = Get-ChildItem -LiteralPath $StateDir -Filter "state-*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not $latest) {
    throw "No remote-test state file found in $StateDir"
  }
  return $latest.FullName
}

function Get-ScopedWinws([string]$Root) {
  $runtimeRoot = (Join-Path $Root "engine-runtime").ToLowerInvariant()
  Get-CimInstance Win32_Process -Filter "Name = 'winws.exe'" -ErrorAction SilentlyContinue |
    Where-Object {
      ($_.ExecutablePath -and $_.ExecutablePath.ToLowerInvariant().Contains($runtimeRoot)) -or
      ($_.CommandLine -and $_.CommandLine.ToLowerInvariant().Contains($runtimeRoot))
    }
}

$statePath = Get-StateFile
$state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
$appPath = [string]$state.app_path
$appPid = [int]$state.app_pid

$process = Get-CimInstance Win32_Process -Filter "ProcessId = $appPid" -ErrorAction SilentlyContinue
if ($process) {
  if ($process.ExecutablePath -ne $appPath) {
    throw "Refusing to stop PID $appPid because executable path does not match state file."
  }
  Stop-Process -Id $appPid -Force -ErrorAction Stop
}

Start-Sleep -Seconds 2

$scopedWinws = @(Get-ScopedWinws -Root $DataRoot)
if ($scopedWinws.Count -gt 0 -and $CleanupScopedWinws) {
  foreach ($item in $scopedWinws) {
    Stop-Process -Id $item.ProcessId -Force -ErrorAction Stop
  }
  Start-Sleep -Seconds 1
  $scopedWinws = @(Get-ScopedWinws -Root $DataRoot)
}

if ($scopedWinws.Count -gt 0) {
  $pids = ($scopedWinws | ForEach-Object { $_.ProcessId }) -join ","
  throw "Scoped ZapretManager winws.exe still running under $DataRoot. PIDs: $pids. Disable in UI first or rerun with -CleanupScopedWinws."
}

[pscustomobject]@{
  ok = $true
  stopped_app_pid = $appPid
  scoped_winws_running = $false
  state_path = $statePath
} | ConvertTo-Json -Depth 4
