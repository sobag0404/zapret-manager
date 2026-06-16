Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Resolve-Path .).Path
$targets = @("target", "app/frontend/dist", "app/frontend/node_modules", "node_modules")
foreach ($target in $targets) {
  $path = Join-Path $root $target
  if (Test-Path -LiteralPath $path) {
    $resolved = (Resolve-Path -LiteralPath $path).Path
    if (-not $resolved.StartsWith($root)) {
      throw "Refusing to remove outside workspace: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
