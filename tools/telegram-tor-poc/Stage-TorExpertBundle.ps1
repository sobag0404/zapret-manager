[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$ManifestPath,
  [Parameter(Mandatory = $true)]
  [string]$ArchivePath,
  [Parameter(Mandatory = $true)]
  [string]$SignaturePath,
  [Parameter(Mandatory = $true)]
  [string]$KeyringPath,
  [Parameter(Mandatory = $true)]
  [string]$GpgvPath,
  [Parameter(Mandatory = $true)]
  [ValidatePattern("^[A-Fa-f0-9]{64}$")]
  [string]$GpgvSha256,
  [Parameter(Mandatory = $true)]
  [string]$ExtractorPath,
  [Parameter(Mandatory = $true)]
  [ValidatePattern("^[A-Fa-f0-9]{64}$")]
  [string]$ExtractorSha256
)

$ErrorActionPreference = "Stop"
$expectedSigner = "EF6E286DDA85EA2A4BA7DE684E2C6E8793298290"

function Convert-ToWindowsCommandLineArgument([string]$Value) {
  if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') {
    return $Value
  }

  $builder = [Text.StringBuilder]::new()
  [void]$builder.Append('"')
  $slashes = 0
  foreach ($character in $Value.ToCharArray()) {
    if ([int][char]$character -eq 92) {
      $slashes++
      continue
    }
    if ($character -eq '"') {
      [void]$builder.Append([string]::new([char]92, ($slashes * 2 + 1)))
      [void]$builder.Append('"')
      $slashes = 0
      continue
    }
    if ($slashes -gt 0) {
      [void]$builder.Append([string]::new([char]92, $slashes))
      $slashes = 0
    }
    [void]$builder.Append($character)
  }
  if ($slashes -gt 0) {
    [void]$builder.Append([string]::new([char]92, ($slashes * 2)))
  }
  [void]$builder.Append('"')
  return $builder.ToString()
}

function Invoke-VerifiedExecutable([string]$FilePath, [string[]]$Arguments) {
  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $FilePath
  $startInfo.Arguments = (($Arguments | ForEach-Object {
    Convert-ToWindowsCommandLineArgument -Value $_
  }) -join ' ')
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true

  $process = [Diagnostics.Process]::new()
  $process.StartInfo = $startInfo
  if (-not $process.Start()) {
    throw "Could not start verified executable: $FilePath"
  }
  $stdoutTask = $process.StandardOutput.ReadToEndAsync()
  $stderrTask = $process.StandardError.ReadToEndAsync()
  $process.WaitForExit()
  [Threading.Tasks.Task]::WaitAll(@($stdoutTask, $stderrTask))
  return [pscustomobject]@{
    ExitCode = $process.ExitCode
    StdOut = $stdoutTask.Result
    StdErr = $stderrTask.Result
  }
}

function Resolve-RequiredFile([string]$Path, [string]$Label) {
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "$Label not found: $Path"
  }
  return (Resolve-Path -LiteralPath $Path).Path
}

$resolvedManifest = Resolve-RequiredFile -Path $ManifestPath -Label "Manifest"
$resolvedArchive = Resolve-RequiredFile -Path $ArchivePath -Label "Archive"
$resolvedSignature = Resolve-RequiredFile -Path $SignaturePath -Label "Signature"
$resolvedKeyring = Resolve-RequiredFile -Path $KeyringPath -Label "Keyring"
$resolvedGpgv = Resolve-RequiredFile -Path $GpgvPath -Label "gpgv"
if (-not [IO.Path]::IsPathFullyQualified($GpgvPath) -or
    [IO.Path]::GetExtension($resolvedGpgv) -ne ".exe") {
  throw "gpgv must be an absolute path to an executable."
}
$resolvedExtractor = Resolve-RequiredFile -Path $ExtractorPath -Label "Extractor"
if (-not [IO.Path]::IsPathFullyQualified($ExtractorPath) -or
    [IO.Path]::GetExtension($resolvedExtractor) -ne ".exe") {
  throw "Extractor must be an absolute path to an executable."
}

$manifest = Get-Content -LiteralPath $resolvedManifest -Raw | ConvertFrom-Json
$requiredProperties = @(
  "schema_version",
  "version",
  "tor_version",
  "platform",
  "source_url",
  "signature_url",
  "sha256",
  "signer_fingerprint"
)
$actualProperties = @($manifest.PSObject.Properties.Name)
$unexpectedProperties = @($actualProperties | Where-Object { $_ -notin $requiredProperties })
$missingProperties = @($requiredProperties | Where-Object { $_ -notin $actualProperties })
if ($unexpectedProperties.Count -gt 0 -or $missingProperties.Count -gt 0) {
  throw "Manifest properties do not match the accepted schema."
}
if ($manifest.schema_version -ne "1" -or $manifest.platform -ne "windows-x86_64") {
  throw "Manifest schema/platform is not accepted."
}
if ($manifest.version -isnot [string] -or [string]::IsNullOrWhiteSpace($manifest.version) -or
    $manifest.tor_version -isnot [string] -or [string]::IsNullOrWhiteSpace($manifest.tor_version)) {
  throw "Manifest versions must be non-empty strings."
}
if ($manifest.signer_fingerprint -ne $expectedSigner) {
  throw "Manifest signer fingerprint is not the pinned Tor Browser Developers key."
}
if ($manifest.source_url -notmatch '^https://(archive|dist)\.torproject\.org/' -or
    $manifest.signature_url -ne "$($manifest.source_url).asc") {
  throw "Manifest source/signature URLs must be matching official Tor Project URLs."
}
if ($manifest.sha256 -notmatch '^[A-Fa-f0-9]{64}$') {
  throw "Manifest SHA-256 is invalid."
}

$archiveName = [Uri]::UnescapeDataString(([Uri]$manifest.source_url).Segments[-1])
if ([IO.Path]::GetFileName($resolvedArchive) -ne $archiveName) {
  throw "Archive filename does not match the pinned source URL."
}

$actualHash = (Get-FileHash -LiteralPath $resolvedArchive -Algorithm SHA256).Hash
if ($actualHash -ne $manifest.sha256.ToUpperInvariant()) {
  throw "Archive SHA-256 does not match the pinned manifest."
}

$gpgvLock = [IO.File]::Open(
  $resolvedGpgv,
  [IO.FileMode]::Open,
  [IO.FileAccess]::Read,
  [IO.FileShare]::Read
)
try {
  $actualGpgvHash = (Get-FileHash -LiteralPath $resolvedGpgv -Algorithm SHA256).Hash
  if ($actualGpgvHash -ne $GpgvSha256.ToUpperInvariant()) {
    throw "gpgv SHA-256 does not match the independently pinned value."
  }
  $signatureResult = Invoke-VerifiedExecutable -FilePath $resolvedGpgv -Arguments @(
    '--status-fd', '1', '--keyring', $resolvedKeyring, $resolvedSignature, $resolvedArchive
  )
  if ($signatureResult.ExitCode -ne 0) {
    throw "Detached signature verification failed."
  }
  $signatureStatus = @($signatureResult.StdOut -split "`r?`n")
} finally {
  $gpgvLock.Dispose()
}
$validFingerprint = $null
foreach ($statusLine in $signatureStatus) {
  $fields = @($statusLine -split ' ')
  if ($fields.Count -ge 3 -and $fields[0] -eq '[GNUPG:]' -and $fields[1] -eq 'VALIDSIG') {
    $primaryFingerprint = $fields[-1]
    if ($primaryFingerprint -match '^[A-Fa-f0-9]{40}$') {
      $validFingerprint = $primaryFingerprint.ToUpperInvariant()
    }
    break
  }
}
if ($validFingerprint -ne $expectedSigner) {
  throw "Detached signature was not made by the pinned Tor Browser Developers key."
}
$postSignatureHash = (Get-FileHash -LiteralPath $resolvedArchive -Algorithm SHA256).Hash
if ($postSignatureHash -ne $actualHash) {
  throw "Archive changed during verification."
}

$localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
if ([string]::IsNullOrWhiteSpace($localAppData)) {
  throw "Windows LocalApplicationData path is unavailable."
}
$managerRoot = Join-Path $localAppData "ZapretManager"
$pocRoot = Join-Path $managerRoot "telegram-tor-poc"
$verifiedRoot = Join-Path $pocRoot "verified-bundles"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if ($null -eq $identity.User) {
  throw "Current Windows identity has no SID."
}

function Set-PrivateDirectoryAcl([string]$Path) {
  $acl = New-Object Security.AccessControl.DirectorySecurity
  $acl.SetOwner($identity.User)
  $acl.SetAccessRuleProtection($true, $false)
  $rule = [Security.AccessControl.FileSystemAccessRule]::new(
    $identity.User,
    [Security.AccessControl.FileSystemRights]::FullControl,
    [Security.AccessControl.InheritanceFlags]"ContainerInherit, ObjectInherit",
    [Security.AccessControl.PropagationFlags]::None,
    [Security.AccessControl.AccessControlType]::Allow
  )
  [void]$acl.AddAccessRule($rule)
  Set-Acl -LiteralPath $Path -AclObject $acl
}

foreach ($ownedPath in @($managerRoot, $pocRoot, $verifiedRoot)) {
  New-Item -ItemType Directory -Path $ownedPath -Force | Out-Null
  $item = Get-Item -LiteralPath $ownedPath -Force
  if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "App-owned verification root must not be a reparse point: $ownedPath"
  }
}
Set-PrivateDirectoryAcl -Path $verifiedRoot
$verifiedRootItem = Get-Item -LiteralPath $verifiedRoot -Force
if (($verifiedRootItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
  throw "Verified bundle root changed during ACL setup."
}

$stageId = "tor-" + ([Guid]::NewGuid().ToString("N"))
$pendingRoot = Join-Path $verifiedRoot ("pending-" + $stageId)
$finalRoot = Join-Path $verifiedRoot ("verified-" + $stageId)
$verifiedPrefix = [IO.Path]::GetFullPath($verifiedRoot).TrimEnd("\") + "\"
$pendingFullPath = [IO.Path]::GetFullPath($pendingRoot)
if (-not $pendingFullPath.StartsWith($verifiedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Pending bundle path escaped the app-owned verification root."
}

New-Item -ItemType Directory -Path $pendingRoot | Out-Null
Set-PrivateDirectoryAcl -Path $pendingRoot
$pendingItem = Get-Item -LiteralPath $pendingRoot -Force
if (($pendingItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
  throw "Pending bundle root changed during ACL setup."
}

$completed = $false
try {
  $extractionRoot = Join-Path $pendingRoot "extracted"
  New-Item -ItemType Directory -Path $extractionRoot | Out-Null

  $stagedArchive = Join-Path $pendingRoot $archiveName
  Copy-Item -LiteralPath $resolvedArchive -Destination $stagedArchive
  $stagedItem = Get-Item -LiteralPath $stagedArchive
  $stagedItem.IsReadOnly = $true
  $stagedHash = (Get-FileHash -LiteralPath $stagedArchive -Algorithm SHA256).Hash
  if ($stagedHash -ne $actualHash) {
    throw "Verified staging copy does not match the signed archive."
  }

  $extractorLock = [IO.File]::Open(
    $resolvedExtractor,
    [IO.FileMode]::Open,
    [IO.FileAccess]::Read,
    [IO.FileShare]::Read
  )
  try {
    $actualExtractorHash = (Get-FileHash -LiteralPath $resolvedExtractor -Algorithm SHA256).Hash
    if ($actualExtractorHash -ne $ExtractorSha256.ToUpperInvariant()) {
      throw "Extractor SHA-256 does not match the independently pinned value."
    }
    $archiveLock = [IO.File]::Open(
      $stagedArchive,
      [IO.FileMode]::Open,
      [IO.FileAccess]::Read,
      [IO.FileShare]::Read
    )
    try {
      $extractResult = Invoke-VerifiedExecutable -FilePath $resolvedExtractor -Arguments @(
        '-xf', $stagedArchive, '-C', $extractionRoot
      )
      if ($extractResult.ExitCode -ne 0) {
        throw "Verified archive extraction failed: $($extractResult.StdErr)"
      }
    } finally {
      $archiveLock.Dispose()
    }
  } finally {
    $extractorLock.Dispose()
  }

  $reparseEntries = @(
    Get-ChildItem -LiteralPath $extractionRoot -Force -Recurse |
      Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 }
  )
  if ($reparseEntries.Count -gt 0) {
    throw "Extracted bundle contains unsupported reparse points."
  }
  $torExecutables = @(Get-ChildItem -LiteralPath $extractionRoot -Filter "tor.exe" -File -Recurse)
  if ($torExecutables.Count -ne 1) {
    throw "Expected exactly one tor.exe in the verified bundle."
  }
  $relativeTorPath = $torExecutables[0].FullName.Substring($pendingRoot.Length).TrimStart("\")
  $torExeHash = (Get-FileHash -LiteralPath $torExecutables[0].FullName -Algorithm SHA256).Hash
  $receipt = [ordered]@{
    status = "verified"
    version = $manifest.version
    tor_version = $manifest.tor_version
    platform = $manifest.platform
    source_url = $manifest.source_url
    archive_sha256 = $actualHash
    signer_fingerprint = $expectedSigner
    gpgv_sha256 = $actualGpgvHash
    extractor_sha256 = $actualExtractorHash
    tor_exe_relative_path = $relativeTorPath
    tor_exe_sha256 = $torExeHash
  }
  $receipt | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $pendingRoot "receipt.json") -Encoding UTF8

  Move-Item -LiteralPath $pendingRoot -Destination $finalRoot
  $completed = $true
  $finalExtractionRoot = Join-Path $finalRoot "extracted"
  $finalArchive = Join-Path $finalRoot $archiveName
  $finalTorPath = Join-Path $finalRoot $relativeTorPath

  [pscustomobject]@{
    verified = $true
    receipt_path = (Join-Path $finalRoot "receipt.json")
    verified_archive_path = $finalArchive
    extraction_root = $finalExtractionRoot
    tor_exe_path = $finalTorPath
    tor_exe_sha256 = $torExeHash
  } | ConvertTo-Json -Depth 3
} finally {
  if (-not $completed -and (Test-Path -LiteralPath $pendingRoot)) {
    Remove-Item -LiteralPath $pendingRoot -Recurse -Force
  }
}
