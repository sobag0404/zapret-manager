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
- `8d3791b engine: add telegram phase0 strategy`
- `2dba720 engine: add runtime web diagnostics`

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
- The static official-CIDR `telegram_web_phase0` candidate also did not change raw TCP or Edge timeout on 2026-07-26. `winws.exe`/WinDivert were active and cleanup passed, so this exact static `syndata,fake,fakedsplit` candidate must not be retried.

## Current Stabilization Changes
- Remote A/B for `2dba720` confirmed that all four runtime DNS-IP candidates
  started `winws`/WinDivert but left Telegram Web and WhatsApp Web at the same
  ~21 s TCP timeout as engine-off. A TCP control to `1.1.1.1:443` passed.
- The active IPv4 addresses were included in each runtime IP set. No response
  SYN,ACK was observed, so `synack-split` and server-window strategies cannot
  act on this path; upstream marks client `--wsize` obsolete.
- The next build adds file-only `winws` debug output to the same four existing
  runtime candidates. Export diagnostics while a candidate is enabled, then Disable removes the runtime directory. No additional bypass strategy is claimed.

- Frontend startup separates critical state from optional diagnostics/update/log calls so one optional failure does not break the main toggle.
- Build Windows workflow now includes `engine/**`, `profiles/**`, `strategies/**`, and manifest/hash tests.
- Engine manifest hash consistency is tested without running binaries.
- Hashed `engine/local/**` and strategy payloads are stored as exact raw bytes via `.gitattributes`; Git must not convert CRLF/LF and invalidate manifests across operating systems.
- Tauri resources test verifies `engine`, `profiles`, and `strategies` are packaged.
- Launch parser tests cover all visible strategies with a runtime path containing spaces.
- Direct launch now unescapes CMD caret escaping, including `^!`, before building argv.
- Launch logs include build provenance: app version and build id.
- Release packaging refuses a dirty worktree and supplies the committed Git id explicitly; CI supplies its checked-out commit id. A release installer must never carry a misleading `-dirty` build id.
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
- The hostlist-only and static official-CIDR Web candidates are deprecated after
  recorded remote failure. The bounded replacement is a manual A/B matrix:
  before launch it resolves a fixed pair of Telegram or WhatsApp Web domains,
  validates public A/AAAA answers, and writes a run-only IP set. Each profile
  has `syndata,fake,fakedsplit` and `wssize=1:6` TCP-443 candidates. Answers
  are limited to eight per domain and sixteen total; unsafe, empty, or
  unexpected answers fail closed. Resolver TTL is unavailable through the
  standard API, so no result persists past Disable. This remains experimental:
  CDN sharing and resolver manipulation are manual-test risks.
- Release packaging now refuses a dirty worktree and passes a verified Git build
  id to Cargo. CI passes its commit id explicitly, preventing a stale `-dirty`
  identity in a clean installer.

## Previous Verified Block

Results below apply to the previous static-CIDR build; the current runtime-DNS
A/B block is not yet built or remotely validated. All service availability
remains unconfirmed.

- `CARGO_BUILD_JOBS=2 cargo fmt --all --check`
- `CARGO_BUILD_JOBS=2 cargo test --workspace`
- `corepack pnpm test`
- `corepack pnpm --dir app/frontend build`
- `CARGO_BUILD_JOBS=2 cargo tauri build`
- Independent read-only Web-strategy review: the Phase 0 wrapper is scoped to
  Telegram alone, uses only the official Telegram CIDRs, TCP 443, and
  `syndata,fake,fakedsplit`; it adds no binaries, services, DNS, proxy, or
  firewall changes. No scope escape was found.

Fresh local test installer for `8d3791b`:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`
- SHA256 `2A330285A5FA2AAA3E9663C1FAE75C8DBB3C3BEE0AD4C600EFA52066A8D48B93`
- Built `2026-07-26 11:29:12` with `CARGO_BUILD_JOBS=2`; embedded build id is
  `8d3791bc2cfe` without a dirty suffix.
- Protected `ZapretManagerSetup.exe` and `ZapretManager v1.0.exe` remain unchanged.

## Verified Current Build

Passed locally for `2dba720` with engine execution disabled:

- `CARGO_BUILD_JOBS=2 cargo fmt --all --check`
- `CARGO_BUILD_JOBS=2 cargo test --workspace` (24 Tauri tests)
- `corepack pnpm test` (18 tests)
- `corepack pnpm --dir app/frontend build`
- `CARGO_BUILD_JOBS=2` release Tauri package build
- Independent read-only review of DNS scope, IP normalization, wrapper argv,
  profile restrictions, and manifest hashes: no command/scope injection found.

Fresh local test installer for `2dba720`:

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`
- SHA256 `9171C01FD93068E9EA90711752994239FE6FDDA89522167B350B68A28E802314`
- Built `2026-07-26 14:49:54` with `CARGO_BUILD_JOBS=2`; embedded build id is
  `2dba720fe6d6` without a dirty suffix.
- Protected `ZapretManagerSetup.exe` and `ZapretManager v1.0.exe` remain unchanged.

GitHub Actions:

- `2f4c8d3`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017561146.
- `2f4c8d3`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017560507.
- `ee8dce4`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372246.
- `ee8dce4`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372233.
- `26a9dd2`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062858.
- `26a9dd2`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062802.
- `8d3791b`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30194597186.
- `8d3791b`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30194597207.

## Manual Test Instructions After Fresh Build

Install the new `ZapretManager v1.2-test.exe` over the old Program Files build. Do not use logs from the old installed app. On the isolated remote PC, record the engine-off baseline, then select Telegram only and test `Runtime IP / Snd`, Disable, and test `Runtime IP / WS`. Repeat with WhatsApp only. Record the actual addresses used by Edge and the run-only `runtime_dns_accepted_ips` list; a browser using a different resolver can select a different endpoint.

For a runtime-IP candidate, export diagnostics **while it is enabled** and only
then press Disable. The export includes the local `winws-debug.log` tail, which
records profile matching without capturing or uploading user traffic. Send this single export together with the remote TCP/Edge timing matrix.

After pressing Enable, if it fails, export diagnostics and send:

- the new `engine-launch.log`;
- `diagnostic-export.txt`;
- the visible build id shown in Diagnostics.

Fresh logs must include `app_version`, `build_id`, `preflight_ok`, `preflight_report`, `argv_list`, `runtime_dns_domains`, `runtime_dns_accepted_ips`, and `runtime_dns_lifetime=run_only`.

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
