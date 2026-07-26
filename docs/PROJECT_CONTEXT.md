# Project Context

Last updated: 2026-07-26

## Current Goal

Zapret Manager v1.2: stabilize enable/disable/diagnostics around the local verified engine before trying new DPI strategies. Discord, YouTube, Telegram, and WhatsApp must be treated as unconfirmed until a fresh build is manually tested.

This is a local Windows app. It is not a VPN, does not use a third-party traffic server, does not require an account, and does not collect telemetry.

## Protected Artifacts

Do not modify, delete, overwrite, or repackage:

- `target/release/bundle/nsis/ZapretManagerSetup.exe`
- `target/release/bundle/nsis/ZapretManager v1.0.exe`

Current test installer:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`

Confirmed local install mismatch:

- Installed `C:\Program Files\Zapret Manager\zapret-manager-tauri.exe`: version `1.2.0`, LastWriteTime `2026-06-26 15:29:14`, SHA256 starts `F00F5755`.
- Previous test installer from `2026-06-29`: SHA256 starts `CA67FB58`.
- A local log from `2026-07-08` without `preflight`, `argv_list`, or `build_id` was created by the old installed app, not by the fresh test line.

## Recent Important Commits

- `351c9f7 docs: update project context`
- `916ff52 engine: log launch argv preflight`
- `e03322b diagnostics: prune strategies and add live status`
- `7820515 engine: disable messaging argv injection`
- `cc1b3ef engine: add messaging profile diagnostics`
- `bc0f094 engine: improve telegram whatsapp diagnostics`
- `229b440 cleanup: keep retry state on failure`
- `afaac6d diagnostics: report unconfirmed checks honestly`
- `d27968a profiles: mark modes experimental`
- `bf6dce6 ci: preserve engine resource hashes`
- `3ef41e5 ci: preserve strategy resource hashes`
- `c2399fc ci: rebuild on attributes changes`
- `a30f562 docs: update project context`
- `c068e58 cleanup: remove app-owned windivert`
- `26a9dd2 engine: add focused web strategies`

## Current Blockers

- Fresh remote evidence on the second PC confirms cleanup: after Disable, scoped `winws.exe=0`, app-owned WinDivert is removed, and runtime directories equal zero.
- Remote strategy matrix on build `ee8dce4`: `general`, `alt`, `alt3`, `simple_fake`, and `fake_tls_auto` matched baseline with no improvement; `alt5` worsened representative targets to TCP 443 failure and exceeded the full probe timeout. All services remain unconfirmed.
- Latest remote strategy matrix was run before the cleanup fix and used only
  general strategies. It does not validate the fresh cleanup installer or the
  new focused Web candidates.
- Old installed build can produce misleading logs; fresh test logs must contain `app_version`, `build_id`, `preflight`, and `argv_list`.
- Strategy status is unknown until validated end-to-end with a live `winws.exe` process and fresh `engine-launch.log`.
- ALT6 is reported broken and must remain hidden/disabled from normal UI/candidates.
- Snapshot/revert for DNS/proxy/firewall is still not implemented; v1.2 only stops the managed engine and cleans runtime state. The app must not claim full DNS/proxy restore.
- General Diagnostics must not claim Windows service, DNS, Internet, Discord, YouTube, Telegram, or WhatsApp are OK without a factual check. Local backend health is separate from Windows service health.
- On 2026-07-26 the separate `telegram_web` and `whatsapp_web` candidates had the same Edge `ERR_CONNECTION_TIMED_OUT` result as engine-off control. `winws.exe` and WinDivert were active, so the hostlist-only TLS path is confirmed too late for this connection path.
- Engine-off DNS returned Telegram `149.154.167.99` and `216.239.32.107`, WhatsApp `31.13.72.52` and `129.134.30.12`. Five-second TCP probes to ports 80/443 did not establish, while public control endpoints connected. This indicates a TCP-establishment filter but does not distinguish phase-0 DPI from route/ACL/IP blocking.

## Current Stabilization Changes

- Frontend startup separates critical state from optional diagnostics/update/log calls so one optional failure does not break the main toggle.
- Build Windows workflow now includes `engine/**`, `profiles/**`, `strategies/**`, and manifest/hash tests.
- Engine manifest hash consistency is tested without running binaries.
- Hashed `engine/local/**` and strategy payloads are stored as exact raw bytes via `.gitattributes`; Git must not convert CRLF/LF and invalidate manifests across operating systems.
- Tauri resources test verifies `engine`, `profiles`, and `strategies` are packaged.
- Launch parser tests cover all visible strategies with a runtime path containing spaces.
- Direct launch now unescapes CMD caret escaping, including `^!`, before building argv.
- Launch logs include build provenance: app version and build id.
- Build id includes `-dirty` when built from uncommitted local changes; final test installer must be rebuilt after the code commit.
- Disable/Exit cleanup keeps enabled state if scoped cleanup fails, so the next action can retry cleanup instead of incorrectly enabling.
- Cleanup failure now reports `RuntimeStatus::Error`; the main toggle shows `Повторить отключение` and calls disable/cleanup again, including orphan-at-start cases.
- Tray Exit closes only after successful scoped cleanup verification; otherwise the app remains open.
- Disable/Exit cleanup now includes scoped app-owned WinDivert driver cleanup: only strict `WinDivert*` service names whose `PathName` is inside `%LOCALAPPDATA%\ZapretManager\engine-runtime` may be stopped/deleted. Unrelated WinDivert services are not touched.
- WinDivert cleanup now uses direct Windows SCM APIs instead of PowerShell/sc.exe. If the app is not elevated, it starts its own executable with a dedicated cleanup CLI flag through UAC, validates the exact runtime root against Windows Known Folder LocalAppData, then verifies that no app-owned WinDivert service remains.
- Cleanup rejects reparse/junction paths and requires canonical driver paths to stay under `%LOCALAPPDATA%\ZapretManager\engine-runtime`.
- WinDivert service matching is case-insensitive but still limited to strict `WinDivert*` names with only alphanumeric, `_`, `.`, and `-`.
- The app now keeps a single-instance Windows mutex, so another Zapret Manager instance cannot race cleanup/start and produce a false disabled status.
- Frontend process restart capability was removed.
- Direct updater install/download-and-install permissions are denied in renderer capabilities. App update check remains available, but auto-install is fail-closed until a Rust-side guarded install command can enforce cleanup before installer handoff.
- `winws.exe` verification is fail-closed: process checks use terminating WMI errors and OpenProcess failures no longer mean “not running”.
- Engine binaries are no longer launched from the user-writable runtime copy. `winws.exe`, WinDivert DLL/SYS, and fake payload files are loaded from bundled `engine/local/bin`; runtime under `%LOCALAPPDATA%` stores only per-run lists/logs.
- Engine manifest/hash validation runs again immediately before direct/UAC launch and logs `prelaunch_hash_ok=true`.
- Elevated process shutdown uses the retained process handle when the app launched through UAC. Scoped orphan cleanup revalidates WMI `ProcessId` + `CreationDate` + command line before terminate and fails closed if a `winws.exe` command line is unreadable.
- Runtime directories are removed only after process and WinDivert cleanup verification succeeds; on failure the runtime is preserved so the next Disable/Exit can retry.
- Stale app-owned WinDivert service entries are still cleaned when their `PathName` is inside the trusted app runtime but the `.sys` file is already missing; scope and reparse checks still run before stop/delete.
- WinDivert cleanup no longer trusts unelevated SCM enumeration as authoritative absence; cleanup elevates through the app-owned helper before stop/delete/verify.
- Scoped orphan `winws.exe` cleanup now requires a path-boundary match for `%LOCALAPPDATA%\ZapretManager\engine-runtime\`, so `engine-runtime-old` and similar paths are not treated as app-owned.
- Diagnostics and diagnostic export include app-owned `WinDivert` driver state so remote testing can verify driver cleanup separately from `winws.exe`.
- Manual snapshot uses the app data root, not `current_dir()`/Program Files.
- Recovery UI and commands now describe only the safe implemented part: stop managed engine and clean runtime state.
- Diagnostics now marks Windows service checks as skipped, reports local backend separately, and treats DNS/Internet/service availability as unconfirmed until explicit health-checks run.
- User-facing profiles Discord/YouTube/Telegram/WhatsApp/Common are marked `experimental` until manual service access is confirmed.
- Remote testing harness is available for a separate Windows 10 PC over SSH/Tailscale. It launches the installed GUI with WebView2 CDP on loopback only through explicit `ZAPRET_MANAGER_REMOTE_TEST_CDP_PORT` handling and does not start the engine by itself.
- Remote baseline from the second PC at 2026-07-23 14:36 MSK, without engine: DNS resolved for all tested services; TCP 443 failed for `web.telegram.org`, `telegram.org`, `web.whatsapp.com`, `www.whatsapp.com`; TCP 443 connected but HTTPS/TLS request failed for `discord.com`, `gateway.discord.gg`, `www.youtube.com`, `i.ytimg.com`; `winws.exe` was not running. This confirms direct blocking before strategy tests and is the expected improvement baseline.
- Remote evidence copied locally outside the repo at `C:\Users\SoBag\Downloads\ZapretManager-remote-diagnostics-20260723-151142` confirms the old cleanup leak: diagnostics retained the first runtime while `WinDivert` was still running after Disable.
- Root cause for the profile UX was confirmed from the old remote launch logs:
  selecting Telegram or WhatsApp was logged but still launched a general
  `general*.bat` command. Two focused experimental candidates now exist:
  `telegram_web` and `whatsapp_web`. Each requires exactly its matching single
  profile and uses only an HTTPS hostlist; it has not yet been remotely proven.
- `alt5` is now deprecated alongside reported-broken `alt6`; neither appears in
  ordinary selection or messaging candidates.
- Focused Web candidates were added in `26a9dd2`: `telegram_web` and
  `whatsapp_web` each use only the matching HTTPS hostlist over TCP 443 and the
  existing audited `fake,fakedsplit` primitive. They reject combined/Common
  profile selection, do not use IP sets or UDP filters, and remain experimental
  until remote testing proves an improvement.
- The hostlist-only Web candidates are now deprecated after the recorded remote
  failure. A single replacement, `telegram_web_phase0`, is scoped to Telegram
  alone and uses the official Telegram CIDR list with `syndata,fake,fakedsplit`
  during TCP establishment. No WhatsApp phase-0 candidate is bundled because
  there is no narrow trusted static WhatsApp Web CIDR list; a broad HTTPS rule
  could affect unrelated traffic.
- Release packaging now refuses a dirty worktree and passes a verified Git build
  id to Cargo. CI passes its commit id explicitly, preventing a stale `-dirty`
  identity in a clean installer.

## Verified In Current Block

Passed locally for `26a9dd2` with engine execution disabled:

- `CARGO_BUILD_JOBS=2 cargo fmt --all --check`
- `CARGO_BUILD_JOBS=2 cargo test --workspace`
- `corepack pnpm test`
- `corepack pnpm --dir app/frontend build`
- `CARGO_BUILD_JOBS=2 cargo tauri build`
- Independent read-only Web-strategy review: scope validation, wrapper parser,
  manifest entries, TCP/hostlist-only invariants, and the absence of new
  binaries/services/DNS/proxy/firewall changes were checked; no scope escape
  found.

Fresh local test installer for `26a9dd2`:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`
- SHA256 `F232E7947251176C34C37213D3FE9558DC2DB9AB4BAFF9B289BF96CE1705A2A5`
- Built `2026-07-24 16:44:42` with `CARGO_BUILD_JOBS=2`.
- Protected `ZapretManagerSetup.exe` and `ZapretManager v1.0.exe` remain unchanged.

GitHub Actions:

- `2f4c8d3`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017561146.
- `2f4c8d3`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017560507.
- `ee8dce4`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372246.
- `ee8dce4`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372233.
- `26a9dd2`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062858.
- `26a9dd2`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062802.

## Manual Test Instructions After Fresh Build

Install the new `ZapretManager v1.2-test.exe` over the old Program Files build. Do not use logs from the old installed app. On the isolated remote test PC, test `Telegram` alone with `Telegram Web`, then `WhatsApp` alone with `WhatsApp Web`; record the engine-off baseline before each test.

After pressing Enable, if it fails, export diagnostics and send:

- the new `engine-launch.log`;
- `diagnostic-export.txt`;
- the visible build id shown in Diagnostics.

Fresh logs must include `app_version`, `build_id`, `preflight_ok`, `preflight_report`, and `argv_list`.

## Remote Test Harness

Use `docs/REMOTE_TESTING.md` and scripts under `tools/remote-test/` for reproducible tests on a separate Windows 10 PC. CDP must stay bound to `127.0.0.1` and be accessed through an SSH tunnel. The coordinator performs real engine/access tests on the remote PC; this repo only provides safe tooling and diagnostics.

## Security Rules

- Never commit secrets, GitHub tokens, `.env`, updater private keys, cookies, or private logs.
- `.tauri-updater/` and signing keys stay local/secret only.
- Do not add or replace engine binaries without trusted source review and `engine/manifest.json` hash updates.
- Do not run third-party scripts or binaries unless reviewed and required.
- Do not log user traffic, private messages, cookies, tokens, passwords, or personal data.
- External engine files are untrusted until manifest/source/hash verification passes.

## Backlog

- Automatic strategy selection by profile health-check after lifecycle stabilization.
- Next strategy-integration block must prioritize Telegram Web and WhatsApp Web first. Desktop apps are second-stage after Web is confirmed by remote tests.
- Focused Web strategy design and its remote test gate are documented in
  `docs/WEB_STRATEGIES.md`.
- Start with manual `Следующая стратегия` and later `Подобрать автоматически`.
- Health-checks only use DNS resolve, TCP connect, and HTTPS connect.
- No user traffic inspection.
- No infinite switching; use cooldown and attempt limits.
