# Telegram Tor PoC Tooling

These files prepare and verify a coordinator-operated remote PoC. They do not
download, install, or launch Tor. The explicit staging command verifies and
extracts the archive without starting `tor.exe`.

## Local Manifest

Create a local JSON manifest outside Git that conforms to
`manifest.schema.json`. Use the exact stable Windows x86_64 archive and `.asc`
URLs from `https://www.torproject.org/download/tor/`. Record the SHA-256 only
after detached-signature verification.

The signer fingerprint must be:

```text
EF6E286DDA85EA2A4BA7DE684E2C6E8793298290
```

Verify and extract the archive into an isolated staging directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\telegram-tor-poc\Stage-TorExpertBundle.ps1 `
  -ManifestPath C:\approved-local\tor-manifest.json `
  -ArchivePath C:\approved-local\tor-expert-bundle-windows-x86_64.tar.gz `
  -SignaturePath C:\approved-local\tor-expert-bundle-windows-x86_64.tar.gz.asc `
  -KeyringPath C:\approved-local\tor.keyring `
  -GpgvPath 'C:\Program Files\GnuPG\bin\gpgv.exe' `
  -GpgvSha256 <independently-recorded-64-character-sha256> `
  -ExtractorPath C:\Windows\System32\tar.exe `
  -ExtractorSha256 <independently-recorded-64-character-sha256>
```

`gpgv.exe` must come from a trusted local installation. Record its SHA-256
independently before the PoC; the verifier does not trust `PATH`. Do the same
for the extractor. The script verifies the signature, re-hashes the archive,
copies it into a current-user-only staging directory, holds the staged archive
open against writes, and extracts it in the same coordinator-invoked operation.

Generate an isolated loopback-only config without launching Tor:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\telegram-tor-poc\New-TelegramTorPocConfig.ps1
```

Use `-ValidateOnly` to check path/port generation without creating files.
The generated free port is not reserved. Start Tor promptly and fail the test
unless the final listener belongs to the recorded Tor PID and is bound only to
`127.0.0.1`.

The coordinator may then explicitly run the `tor_exe_path` returned by the
staging command:

```powershell
& 'C:\Users\<user>\AppData\Local\ZapretManager\telegram-tor-poc\verified-bundles\<stage>\extracted\tor\tor.exe' `
  -f 'C:\Users\<user>\AppData\Local\ZapretManager\telegram-tor-poc\<run>\torrc'
```

Do not automate this command in production. Record the exact PID, executable
path/hash, creation time, run directory, and SOCKS port before testing.

## Verification Checklist

1. Download archive and `.asc` only from the official Tor Project page.
2. Verify the key fingerprint, detached signature, pinned archive SHA-256, and
   independently pinned `gpgv.exe` and extractor SHA-256 values.
3. Use only the `tor_exe_path` returned by the atomic stage/extract command.
4. Confirm Windows 10 18363 can run `tor.exe --version`.
5. Confirm the listener is only `127.0.0.1:<port>` and belongs to the recorded
   Tor PID.
6. Wait for bootstrap 100%; retain only redacted bootstrap status.
7. Confirm direct baseline to `https://web.telegram.org/` still fails.
8. Confirm `curl --proxy socks5h://127.0.0.1:<port>
   https://web.telegram.org/` succeeds without local DNS resolution.
9. For the optional browser test, use a separate disposable Edge/WebView2
   profile, SOCKS per-process, QUIC disabled, and no extensions. Treat this as
   access-only, not anonymity.
10. Observe Windows DNS and process sockets during the browser test. Any direct
    Telegram or DNS socket fails the PoC.
11. Stop only the recorded Tor PID. Confirm the process and SOCKS listener are
    gone before deleting the isolated run directory.
12. Telegram Desktop may be tested only through its visible supported proxy UI.
    Do not edit private configuration. Record whether manual rollback is
    possible; otherwise keep the product UX Web-only.

Do not share raw Tor logs. They may contain local paths or network metadata even
with safe logging enabled.
