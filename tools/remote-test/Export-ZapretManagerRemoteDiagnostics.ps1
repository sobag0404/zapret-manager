param(
  [string]$DataRoot = (Join-Path $env:LOCALAPPDATA "ZapretManager"),
  [string]$OutputRoot = (Join-Path $env:LOCALAPPDATA "ZapretManager\remote-test\exports"),
  [string]$AppPath = "C:\Program Files\Zapret Manager\zapret-manager-tauri.exe",
  [int]$LatestEngineLogs = 5
)

$ErrorActionPreference = "Stop"

function Get-ScopedWinws([string]$Root) {
  $runtimeRoot = (Join-Path $Root "engine-runtime").ToLowerInvariant()
  Get-CimInstance Win32_Process -Filter "Name = 'winws.exe'" -ErrorAction SilentlyContinue |
    Where-Object {
      ($_.ExecutablePath -and $_.ExecutablePath.ToLowerInvariant().Contains($runtimeRoot)) -or
      ($_.CommandLine -and $_.CommandLine.ToLowerInvariant().Contains($runtimeRoot))
    } |
    Select-Object ProcessId, ExecutablePath, CommandLine, CreationDate
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outDir = Join-Path $OutputRoot $stamp
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$summaryPath = Join-Path $outDir "diagnostic-export.txt"
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("Zapret Manager remote diagnostic export")
$lines.Add("timestamp=$((Get-Date).ToString('o'))")
$lines.Add("data_root=$DataRoot")
$lines.Add("app_path=$AppPath")

if (Test-Path -LiteralPath $AppPath -PathType Leaf) {
  $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $AppPath
  $version = (Get-Item -LiteralPath $AppPath).VersionInfo.ProductVersion
  $lines.Add("app_sha256=$($hash.Hash)")
  $lines.Add("app_product_version=$version")
} else {
  $lines.Add("app_missing=true")
}

$appProcesses = Get-CimInstance Win32_Process -Filter "Name = 'zapret-manager-tauri.exe'" -ErrorAction SilentlyContinue |
  Select-Object ProcessId, ExecutablePath, CommandLine, CreationDate
$scopedWinws = @(Get-ScopedWinws -Root $DataRoot)

$lines.Add("")
$lines.Add("[zapret-manager-tauri.exe]")
$lines.Add(($appProcesses | ConvertTo-Json -Depth 4))
$lines.Add("")
$lines.Add("[scoped winws.exe under data_root engine-runtime]")
$lines.Add(($scopedWinws | ConvertTo-Json -Depth 4))

$logDir = Join-Path $DataRoot "logs"
if (Test-Path -LiteralPath $logDir) {
  Copy-Item -LiteralPath $logDir -Destination (Join-Path $outDir "logs") -Recurse -Force
}

$runtimeRoot = Join-Path $DataRoot "engine-runtime"
if (Test-Path -LiteralPath $runtimeRoot) {
  $engineLogs = Get-ChildItem -LiteralPath $runtimeRoot -Recurse -Filter "engine-launch.log" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First $LatestEngineLogs
  $engineOut = Join-Path $outDir "engine-launch-logs"
  New-Item -ItemType Directory -Force -Path $engineOut | Out-Null
  foreach ($log in $engineLogs) {
    $safeName = ($log.Directory.Name + "-" + $log.Name)
    Copy-Item -LiteralPath $log.FullName -Destination (Join-Path $engineOut $safeName) -Force
    $lines.Add("engine_launch_log=$($log.FullName)")
  }
}

$lines | Set-Content -LiteralPath $summaryPath -Encoding UTF8

[pscustomobject]@{
  ok = $true
  output_dir = $outDir
  diagnostic_export = $summaryPath
  scoped_winws_count = $scopedWinws.Count
} | ConvertTo-Json -Depth 4
