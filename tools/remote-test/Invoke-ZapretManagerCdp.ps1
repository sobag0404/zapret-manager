param(
  [ValidateRange(1024, 65535)]
  [int]$CdpPort = 9223,
  [Parameter(Mandatory)]
  [string]$Expression,
  [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"

function Receive-CdpMessage([System.Net.WebSockets.ClientWebSocket]$Socket, [int]$TimeoutSeconds) {
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $buffer = New-Object byte[] 65536
  $stream = New-Object System.IO.MemoryStream
  try {
    do {
      $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalSeconds)
      $cts = New-Object System.Threading.CancellationTokenSource
      $cts.CancelAfter([TimeSpan]::FromSeconds($remaining))
      $result = $Socket.ReceiveAsync([ArraySegment[byte]]::new($buffer), $cts.Token).GetAwaiter().GetResult()
      if ($result.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
        throw "CDP connection closed before a response was received."
      }
      $stream.Write($buffer, 0, $result.Count)
    } while (-not $result.EndOfMessage)
    return [Text.Encoding]::UTF8.GetString($stream.ToArray())
  } finally {
    $stream.Dispose()
  }
}

$target = Invoke-RestMethod -Uri "http://127.0.0.1:$CdpPort/json/list" -TimeoutSec $TimeoutSeconds |
  Where-Object { $_.type -eq "page" } |
  Select-Object -First 1
if (-not $target -or [string]::IsNullOrWhiteSpace($target.webSocketDebuggerUrl)) {
  throw "No inspectable Zapret Manager page is available on CDP port $CdpPort."
}

$socket = New-Object System.Net.WebSockets.ClientWebSocket
$cts = New-Object System.Threading.CancellationTokenSource
try {
  $cts.CancelAfter([TimeSpan]::FromSeconds($TimeoutSeconds))
  $socket.ConnectAsync([Uri]$target.webSocketDebuggerUrl, $cts.Token).GetAwaiter().GetResult()
  $request = @{ id = 1; method = "Runtime.evaluate"; params = @{ expression = $Expression; returnByValue = $true; awaitPromise = $true } } |
    ConvertTo-Json -Depth 8 -Compress
  $bytes = [Text.Encoding]::UTF8.GetBytes($request)
  $socket.SendAsync(
    [ArraySegment[byte]]::new($bytes),
    [System.Net.WebSockets.WebSocketMessageType]::Text,
    $true,
    [Threading.CancellationToken]::None
  ).GetAwaiter().GetResult()

  do {
    $message = Receive-CdpMessage -Socket $socket -TimeoutSeconds $TimeoutSeconds | ConvertFrom-Json
  } while ($message.id -ne 1)
  if ($message.error) {
    throw "CDP Runtime.evaluate failed: $($message.error.message)"
  }
  if ($message.result.exceptionDetails) {
    throw "Page evaluation failed: $($message.result.exceptionDetails.text)"
  }
  $message.result.result.value | ConvertTo-Json -Depth 12
} finally {
  if ($socket.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
    $socket.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "done", [Threading.CancellationToken]::None).GetAwaiter().GetResult()
  }
  $socket.Dispose()
  $cts.Dispose()
}
