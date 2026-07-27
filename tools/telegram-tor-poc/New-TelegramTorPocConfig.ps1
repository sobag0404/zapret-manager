[CmdletBinding()]
param(
  [ValidateRange(0, 65535)]
  [int]$SocksPort = 0,
  [switch]$ValidateOnly
)

$ErrorActionPreference = "Stop"
$localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
if ([string]::IsNullOrWhiteSpace($localAppData)) {
  throw "Windows LocalApplicationData path is unavailable."
}
$managerRoot = Join-Path $localAppData "ZapretManager"
$baseRoot = Join-Path $managerRoot "telegram-tor-poc"
$templatePath = Join-Path $PSScriptRoot "torrc.template"

if (-not (Test-Path -LiteralPath $templatePath -PathType Leaf)) {
  throw "torrc template not found: $templatePath"
}

function Get-AvailableLoopbackPort {
  $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
  try {
    $listener.Start()
    return ([Net.IPEndPoint]$listener.LocalEndpoint).Port
  } finally {
    $listener.Stop()
  }
}

foreach ($ownedPath in @($managerRoot, $baseRoot)) {
  if (Test-Path -LiteralPath $ownedPath) {
    $item = Get-Item -LiteralPath $ownedPath -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "App-owned PoC root must not be a reparse point: $ownedPath"
    }
  }
}

if ($SocksPort -eq 0) {
  $SocksPort = Get-AvailableLoopbackPort
}
if ($SocksPort -lt 1024) {
  throw "SOCKS port must be unprivileged."
}

$occupied = Get-NetTCPConnection -LocalPort $SocksPort -State Listen -ErrorAction SilentlyContinue
if ($occupied) {
  throw "SOCKS port $SocksPort is already listening."
}

$runId = "telegram-tor-poc-" + (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + ([Guid]::NewGuid().ToString("N").Substring(0, 8))
$sessionRoot = Join-Path $baseRoot $runId
$dataRoot = Join-Path $sessionRoot "data"
$cacheRoot = Join-Path $sessionRoot "cache"
$logRoot = Join-Path $sessionRoot "logs"
$torrcPath = Join-Path $sessionRoot "torrc"
$statePath = Join-Path $sessionRoot "state.json"

if ($ValidateOnly) {
  [pscustomobject]@{
    valid = $true
    socks_endpoint = "socks5://127.0.0.1:$SocksPort"
    session_root = $sessionRoot
    note = "Validation only; no directory was created and Tor was not launched."
  } | ConvertTo-Json -Depth 3
  exit 0
}

New-Item -ItemType Directory -Force -Path $dataRoot, $cacheRoot, $logRoot | Out-Null

function Convert-ToTorPath([string]$Path) {
  return ([IO.Path]::GetFullPath($Path)).Replace("\", "/")
}

$torrc = Get-Content -LiteralPath $templatePath -Raw
$torrc = $torrc.Replace("{{SOCKS_PORT}}", $SocksPort.ToString())
$torrc = $torrc.Replace("{{DATA_DIRECTORY}}", (Convert-ToTorPath $dataRoot))
$torrc = $torrc.Replace("{{CACHE_DIRECTORY}}", (Convert-ToTorPath $cacheRoot))
$torrc = $torrc.Replace("{{PID_FILE}}", (Convert-ToTorPath (Join-Path $sessionRoot "tor.pid")))
$torrc = $torrc.Replace("{{NOTICE_LOG}}", (Convert-ToTorPath (Join-Path $logRoot "notice.log")))
if ($torrc -match '\{\{[A-Z_]+\}\}') {
  throw "torrc contains unresolved placeholders."
}
$torrc | Set-Content -LiteralPath $torrcPath -Encoding UTF8

[pscustomobject]@{
  schema_version = "1"
  run_id = $runId
  created_at = (Get-Date).ToString("o")
  session_root = $sessionRoot
  torrc_path = $torrcPath
  socks_host = "127.0.0.1"
  socks_port = $SocksPort
  tor_pid = $null
  tor_exe_path = $null
  tor_exe_sha256 = $null
} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $statePath -Encoding UTF8

Get-Content -LiteralPath $statePath -Raw
