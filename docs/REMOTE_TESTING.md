# Remote Testing Harness

This harness is for a separate Windows 10 test PC accessed over SSH/Tailscale. It does not change production behavior and does not enable the engine by itself.

## What It Does

- Starts the installed Zapret Manager GUI in an interactive Windows session.
- Enables WebView2 CDP only for that test process using an explicit Zapret Manager test env var and WebView2 environment fallback.
- Binds CDP to `127.0.0.1` only.
- Lets the coordinator connect Playwright through an SSH local port-forward.
- Stops only the Zapret Manager process launched by the harness.
- Verifies that no ZapretManager-owned `winws.exe` remains under the local app data runtime root.
- Exports logs and launch diagnostics into a local test export folder.

WebView2 supports `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` and programmatic `AdditionalBrowserArguments`. Zapret Manager uses the programmatic path only when `ZAPRET_MANAGER_REMOTE_TEST_CDP_PORT` is set to a valid port. Production starts without CDP when that env var is absent. Playwright connects through CDP with `chromium.connectOverCDP(...)`.

References:

- Microsoft WebView2 environment variables: https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl
- Playwright WebView2 CDP testing: https://playwright.dev/docs/webview2

## Remote PC Preconditions

- Zapret Manager is installed, usually at:
  `C:\Program Files\Zapret Manager\zapret-manager-tauri.exe`
- A real user is logged into the Windows desktop session.
- SSH access works over Tailscale or another private channel.
- The app starts with engine disabled before each test.
- Do not run this on the main PC while VPN-sensitive work is active.

## Start App With CDP

Run on the remote Windows PC over SSH:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Start-ZapretManagerCdp.ps1 -CdpPort 9223
```

If the scheduled task launch is not available, use direct launch from an already interactive shell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Start-ZapretManagerCdp.ps1 -CdpPort 9223 -LaunchMode Direct
```

The script prints a JSON object with:

- `run_id`
- `app_pid`
- `cdp_endpoint`
- `ssh_tunnel`
- `state_path`

## SSH Tunnel From Coordinator Machine

Run locally on the coordinator machine:

```bash
ssh -L 9223:127.0.0.1:9223 <windows-test-user>@<tailscale-host>
```

Then connect Playwright to:

```text
http://127.0.0.1:9223
```

Do not expose the CDP port on a public interface.

## Safety Flow For Real Engine Tests

Before each strategy:

1. Export diagnostics baseline.
2. Confirm the app status is disabled.
3. Confirm no scoped `winws.exe` is running under `%LOCALAPPDATA%\ZapretManager\engine-runtime`.
4. Start the test through the UI.
5. Save `engine-launch.log` and diagnostics export.
6. Press `Выключить` in the UI.
7. Verify no scoped `winws.exe` remains.
8. If cleanup fails, do not mark the strategy as working.

The stop script refuses to kill unrelated `winws.exe` processes. It only checks processes whose executable path or command line points into Zapret Manager's app-owned runtime root.

## Export Diagnostics

Run on the remote Windows PC:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Export-ZapretManagerRemoteDiagnostics.ps1
```

The script writes:

- `diagnostic-export.txt`
- copied user/debug logs
- latest `engine-launch.log` files
- app process summary
- scoped `winws.exe` process summary

Review the export before sharing it. It must not contain passwords, cookies, tokens, or user traffic.

## Stop Harness App

After disabling through the UI, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Stop-ZapretManagerCdp.ps1
```

If a scoped ZapretManager `winws.exe` still remains after UI disable, collect diagnostics first. Only then, for cleanup on the test PC:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Stop-ZapretManagerCdp.ps1 -CleanupScopedWinws
```

This still refuses to touch `winws.exe` outside Zapret Manager's runtime root.

## Validation Without Launch

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\remote-test\Start-ZapretManagerCdp.ps1 -ValidateOnly
```

Use this on the main PC or CI-style checks. It does not start Zapret Manager and does not touch the engine.
