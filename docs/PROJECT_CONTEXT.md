# Project Context

Last updated: 2026-07-28

## Current Goal

Zapret Manager v1.2: stabilize enable/disable/diagnostics around the local verified engine. Discord, Telegram, and WhatsApp remain unconfirmed. YouTube Web has limited test-network evidence only through the existing experimental `fake_tls_auto` variant.

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
- `5f10823 engine: add runtime debug diagnostics`
- `7914fcd engine: add telegram syn discriminator`
- `16cd76d ui: recommend youtube strategy`
- `7845808 launcher: restore timestamp prerequisite`
- `4fc6bbc launcher: harden timestamp rollback`

## Current Blockers

- Fresh remote evidence on the second PC confirms cleanup: after Disable, scoped `winws.exe=0`, app-owned WinDivert is removed, and runtime directories equal zero.
- A newer Windows 10 remote test of installer `F59A3E3E5F3A797FE0ACF1103B372ABB4612146188D110A33326BA227A69D524` confirmed YouTube Web only with the existing experimental `fake_tls_auto`: HTTP 200, the main page and `jNQXAC9IVRw` watch page loaded, `readyState=4`, and playback advanced from 6.12 to 13.12 seconds in seven seconds. This is evidence for that test network only, not a stable claim.
- The same test found Discord Web reset/TLS errors on existing variants 2/4/5/6. Official Discord Desktop `1.0.9249` remained at `Checking for updates…` after isolated 35–90 second restarts. Discord remains experimental and unconfirmed.
- `fake_tls_auto` allowed the official Discord installer download through winget with hash verification, but did not make Discord connect.
- Fresh remote validation of installer SHA256 `F59A3E3E5F3A797FE0ACF1103B372ABB4612146188D110A33326BA227A69D524` on Windows 10 18363 confirmed that `telegram_web_runtime_dup` left Edge `web.telegram.org` at `ERR_CONNECTION_TIMED_OUT` (~21.19 s), matching the engine-off baseline (~21.21 s). A target TCP probe to `149.154.167.99:443` timed out (~6.04 s) while `1.1.1.1:443` passed (~52 ms). The prior runtime IP-set/profile-match evidence plus no inbound SYN,ACK make this a strong destination-IP/SYN-stage filtering signal, not a TLS/SNI or profile-selection result.
- The same fresh validation confirmed safe shutdown: app, `winws.exe`, Edge test process, and app-owned WinDivert were absent after Disable; runtime directories equalled zero.
- On this test network the currently bundled local DPI-only candidates have no confirmed path to Telegram Web. Further packet-modification experiments are intentionally paused; the product must not claim Telegram availability without a changed route/network condition and a new remote proof.
- Old installed build can produce misleading logs; fresh test logs must contain `app_version`, `build_id`, `preflight`, and `argv_list`.
- Strategy status is unknown until validated end-to-end with a live `winws.exe` process and fresh `engine-launch.log`.
- Legacy ALT6 is reported broken and remains hidden/disabled. It is distinct from the existing experimental UI option numbered `6 Fake TLS Auto`.
- Snapshot/revert for DNS/proxy/firewall is still not implemented; v1.2 only stops the managed engine and cleans runtime state. The app must not claim full DNS/proxy restore.
- General Diagnostics must not claim Windows service, DNS, Internet, Discord, YouTube, Telegram, or WhatsApp are OK without a factual check. Local backend health is separate from Windows service health.
- On 2026-07-26 the separate `telegram_web` and `whatsapp_web` candidates had the same Edge `ERR_CONNECTION_TIMED_OUT` result as engine-off control. `winws.exe` and WinDivert were active, so the hostlist-only TLS path is confirmed too late for this connection path.
- Engine-off DNS returned Telegram `149.154.167.99` and `216.239.32.107`, WhatsApp `31.13.72.52` and `129.134.30.12`. Five-second TCP probes to ports 80/443 did not establish, while public control endpoints connected. This indicates a TCP-establishment filter but does not distinguish phase-0 DPI from route/ACL/IP blocking.
- The static official-CIDR `telegram_web_phase0` candidate also did not change raw TCP or Edge timeout on 2026-07-26. `winws.exe`/WinDivert were active and cleanup passed, so this exact static `syndata,fake,fakedsplit` candidate must not be retried.
- Discord remains unconfirmed. Remote Windows 10 evidence for existing variants 2/4/5/6 was Discord Web reset/TLS protocol errors and Discord Desktop `1.0.9249` staying at `Checking for updates…`; cleanup passed after every run.

## Current Stabilization Changes
- Remote validation of installer `3331F1...` on Windows 10 confirmed the
  existing `alt` strategy (UI `2 ALT`) for Discord Web and Desktop: Web curl
  returned HTTP 200, headless Edge loaded a Discord title without an error,
  and Discord Desktop `1.0.9249` completed updater/module initialization and
  the remote-auth WSS hello handshake. Disable cleanup passed with `winws=0`,
  no running app-owned WinDivert, zero runtime directories, and TCP timestamps
  restored to disabled. This is test-network evidence, not a global stable
  claim.
- The UI now recommends existing `alt` for Discord-only selection when the
  engine is disabled and the user has not made a manual strategy choice. It
  does not replace manual selections, running/error state, mixed profiles, or
  the existing YouTube-only `fake_tls_auto` recommendation. `2 ALT` is labeled
  experimental and verified on the test network for Discord Web/Desktop.
- The remote Discord-only test of installer `D3C2312F...` reached the fail-closed
  TCP-timestamps prerequisite but could not parse `netsh interface tcp show
  global`, although the same Windows 10 host reported `RFC 1323 Timestamps :
  disabled` when run interactively. The reader now captures raw bytes, tries
  valid UTF-8 plus the active ANSI/OEM Windows code pages through
  `MultiByteToWideChar`, and parses the locale-invariant `RFC 1323` row with
  English and Russian enabled/disabled values. The prerequisite remains
  fail-closed when the row or state is unknown. No engine arguments or engine
  files changed.
- The direct `winws.exe` transition had skipped `service.bat status_zapret` from v1.0. The audited BAT invokes it before every launch and enables Windows TCP timestamps, while the direct launcher previously did not. For existing arguments containing `--dpi-desync-fooling=ts`, the app now temporarily restores that prerequisite through a narrow UAC helper, records a runtime lease, and restores the prior disabled state on Disable/Exit or a recovered stale lease. It does not alter `fake_tls_auto`, which has no `ts` argument.
- Offline parity tests use the existing Discord command files with a path containing spaces and verify direct argv preserves quoted path token boundaries, repeated `--new` filters, hostlists, ipsets, and the legacy timestamp prerequisite. No engine file, manifest hash, or DPI argument value changed.
- When only YouTube is selected while the engine is disabled and the default strategy is still active, the UI selects the existing experimental `fake_tls_auto` recommendation. A saved manual strategy or an active/error engine is never changed automatically.
- The UI labels `6 Fake TLS Auto` as test-network evidence for YouTube Web and distinguishes it from hidden legacy ALT6. This does not change any engine files, manifest hashes, or network parameters.
- Remote A/B for `2dba720` confirmed that all four runtime DNS-IP candidates
  started `winws`/WinDivert but left Telegram Web and WhatsApp Web at the same
  ~21 s TCP timeout as engine-off. A TCP control to `1.1.1.1:443` passed.
- The active IPv4 addresses were included in each runtime IP set. No response
  SYN,ACK was observed, so `synack-split` and server-window strategies cannot
  act on this path; upstream marks client `--wsize` obsolete.
- Remote debug evidence from `5f10823` confirms that the generated IPv4 IP sets match Telegram and WhatsApp targets and that `winws` applies the Phase-0 profile. In `syndata`, `winws` intentionally replaces the original SYN with a SYN carrying data; it is expected to drop that intercepted original rather than immediately send a second ordinary SYN. No target SYN,ACK was observed.
- Persistent `--debug` was removed from all normal runtime candidates because its global TCP-443 capture produced hundreds of KB of output and once delayed an unrelated control connection. Standard diagnostics now retain only launch/preflight, runtime IP-set, process, and cleanup evidence.
- The Telegram-only `telegram_web_runtime_dup` diagnostic candidate was tested on the same network and did not change the timeout or produce a SYN,ACK. It remains experimental evidence only and is not presented as a working access strategy.

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

- `4fc6bbc`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30264107702.
- `4fc6bbc`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30264107751.
- `c595c8c`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30264157288.
- `16cd76d`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30262510728.
- `16cd76d`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30262510731.
- `1f0ba56`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30262660462.
- `5f10823`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30243787596.
- `5f10823`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30243787592.
- 9e3a726 (code build 7914fcd): CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30253649496.
- 9e3a726 (code build 7914fcd): Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30253649558.


- `2f4c8d3`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017561146.
- `2f4c8d3`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30017560507.
- `ee8dce4`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372246.
- `ee8dce4`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30004372233.
- `26a9dd2`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062858.
- `26a9dd2`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30079062802.
- `8d3791b`: CI passed, https://github.com/sobag0404/zapret-manager/actions/runs/30194597186.
- `8d3791b`: Build Windows passed, https://github.com/sobag0404/zapret-manager/actions/runs/30194597207.

## Latest Test Installer

- `target/release/bundle/nsis/ZapretManager v1.2-test.exe`
- SHA256 `EE972DA4665BA75FE233DD9CD563715259C95074B6EE89AE1F18F0FBCD9AE384`
- Built from clean `df0cbaa` on `2026-07-27` with `CARGO_BUILD_JOBS=2`.
- Protected `ZapretManagerSetup.exe` and `ZapretManager v1.0.exe` retain SHA256 `612B4D42507888E25387CEF4658C62E9021D1BD41EE4C26DAD48398D56FD6D52`.

## Latest Stabilization Build

- Commit `be96f58`: `ci: make codepage fixture deterministic`.
- The previous CI failure was caused by a test using the machine's active OEM
  code page to decode a CP866 fixture. The test now selects CP866 explicitly;
  production still uses the active Windows ANSI/OEM pages.
- The build reads redirected `netsh` output as raw bytes and decodes active
  ANSI/OEM code pages before parsing the RFC 1323 state. No engine files or
  network arguments changed. The embedded build id is `01e0080`.
- Local checks passed: `cargo fmt --all --check`, `cargo test --workspace`
  (35 tests), `corepack pnpm test` (20 tests), frontend production build, and
  Tauri release/NSIS build. Engine was not launched on the main PC.
- Fresh installer SHA256:
  `3331F1B2844EC87735DF53A02BC53054F7794B989970ABFD1156631F960F933B`.
- GitHub Actions for `01e0080`/`61ab33e` failed in `Rust workspace tests`;
  Build Windows for `01e0080` passed. The replacement commit is `be96f58`;
  its CI and Build Windows runs are the release gate before remote testing.

## Latest UX Build

- Commit `df0cbaa`: `ui: recommend verified Discord strategy`.
- Discord-only selection now recommends existing `alt` only while disabled,
  with no manual strategy choice and no mixed profiles. YouTube behavior is
  unchanged.
- `2 ALT` remains experimental and is labeled as verified on the test network
  for Discord Web/Desktop only.
- Final remote smoke for installer SHA
  `EE972DA4665BA75FE233DD9CD563715259C95074B6EE89AE1F18F0FBCD9AE384` on
  Windows 10 passed: after resetting to `general` and reloading, selecting
  Discord alone automatically changed the UI to `2 ALT`; engine enabled
  without error; Discord Web returned HTTP 200; Discord Desktop `1.0.9249`
  reached `launching`, BUILD INFO, and completed the remote-auth WSS hello
  handshake with six processes. The launch log recorded the timestamp
  prerequisite and `enabled_by_manager=true`. Disable completed without error:
  `winws=0`, running WinDivert=0, runtime directories=0, and TCP timestamps
  returned to disabled. Discord, Edge, harness, and tunnel were stopped. This
  remains test-network evidence and is not a global stable claim.

## Latest PoC Tooling

- Commit `0029a5e`: `tools: harden Tor PoC verification`.
- CI passed: https://github.com/sobag0404/zapret-manager/actions/runs/30393136343.
- Build Windows passed: https://github.com/sobag0404/zapret-manager/actions/runs/30393136392.
- No product UI, engine parameters, bundled binaries, or protected installers
  changed in this block.

## Manual Test Instructions After Fresh Build

Install the new `ZapretManager v1.2-test.exe` over the old Program Files build only for lifecycle validation. Do not treat the current Telegram runtime candidates as an availability fix on the tested network, and do not change WhatsApp strategies in this validation block. Record the engine-off baseline and only the launch/cleanup state needed to confirm process, driver, and runtime removal.

For YouTube-only validation, leave the engine disabled, select only YouTube, and confirm that the existing `6 Fake TLS Auto` is selected as the test-network recommendation. It must remain marked experimental. Test the main page and one watch page, then press Disable and confirm app-owned `winws.exe`, WinDivert, and runtime directories are absent. Select Discord separately only to verify that no availability claim is shown; do not treat it as working without a new remote proof.

For Discord-only validation, start with engine disabled and verify `netsh interface tcp show global` reports TCP timestamps disabled. Select Discord only and test existing variants 2, 4, and 5 separately. For each variant, approve the temporary UAC request, confirm `engine-launch.log` reports `legacy_bat_tcp_timestamps_required=true` and the managed state, then test Discord Web and a clean Discord Desktop restart. Press Disable after each attempt and verify `winws.exe=0`, no app-owned WinDivert, runtime directories equal zero, and TCP timestamps return to disabled. Variant 6 is a control: it must not request or log the timestamp prerequisite because its existing argv has no `ts` value.

For a runtime-IP candidate, record the launch log and runtime DNS IP list, run the
scoped TCP/Edge probe, then press Disable. Normal candidates do not enable packet
`--debug`; diagnostics contain launch/preflight, process, IP-set, and cleanup
metadata only and are never uploaded automatically.

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
- Telegram DPI-only work is closed for the tested ISP because destination TCP
  443 is blocked before TLS/SNI and the audited packet-level candidates did not
  produce a SYN,ACK. `docs/ADR-0001-TELEGRAM-TOR-POC.md` defines a separate,
  explicit-opt-in feasibility PoC using a verified official Tor Expert Bundle
  as a loopback-only Telegram Web SOCKS route. This changes the trust model:
  traffic uses Tor relays, ordinary WebView2 is not equivalent to Tor Browser,
  and no production integration is approved. The repository contains only
  offline artifact/config verification tooling; no Tor binary, network
  downloader, or Tor launcher was added.
- Final verifier bootstrap and vanilla Tor PoC on Windows 10 18363 completed
  its safety gates. Official Gpg4win 5.0.2 was resumed only from
  `files.gpg4win.org` with the same ETag; its published SHA-256 was
  `11864cdc6dedd58c5448ab1c0868886e56bdad96972bc06dcd44b80f9e527051` and
  Authenticode signer was `g10 Code GmbH`. The identical installer was copied
  to the test PC, rehashed, installed silently, and the installed `gpgv.exe`
  SHA-256 was `a79aa4d953298a3fcecd50634c4d5d2a66606587dce38d66dd7dfdf66a30c3f0`.
- The official Tor Expert Bundle 15.0.19 archive SHA-256 was
  `6ac067402c7b4a3dc37887ed3754b3914b67fdc220c966190683e9ccf91abf0f`.
  Its detached signature passed `VALIDSIG` for the pinned Tor Browser
  Developers primary fingerprint `EF6E286DDA85EA2A4BA7DE684E2C6E8793298290`;
  isolated staging found exactly one `tor.exe` and no reparse points. The
  bundle was never added to Git or product resources.
- The verified vanilla Tor process bound only its app-owned loopback SOCKS
  session, but did not reach bootstrap 100% within 180 seconds. It remained at
  10% with repeated relay TLS `CONNECTRESET` and timeout failures. Therefore
  no SOCKS curl or Edge Telegram Web access claim was made. This is a network
  blocker for vanilla Tor, not evidence that Telegram Web is available.
- Cleanup passed after the failed bootstrap: `tor=0`, Tor listeners=0,
  Edge=0, and `winws.exe=0`; the entire app-owned PoC runtime and transferred
  inputs were removed. Global proxy remains disabled with no server or PAC.
  The test tool has no code path to modify Windows DNS or firewall; their
  current state was read-only recorded. Gpg4win remains installed only as the
  verified local signature tool.
- PoC tooling was corrected for Windows: `gpgv` stderr is captured without
  treating successful diagnostics as PowerShell failures, `VALIDSIG` is
  checked against its primary fingerprint field, generated `torrc` is UTF-8
  without BOM, and Tor notice logs use stdout redirected by the test launcher
  instead of a path-sensitive torrc file sink.
- Vanilla Tor must not be retried automatically on this test network. The only
  safe next feasibility step is a separately approved official Tor bridge or
  obfs4 configuration with a user-provided official bridge line. It remains
  outside product integration and must repeat the same loopback, leak, and
  cleanup gates.
- The limited bridge/pluggable-transport follow-up was closed without launching
  a transport. Official Tor documentation confirms that the Expert Bundle
  contains `lyrebird` and that it implements obfs4, Snowflake, and WebTunnel;
  Snowflake has an official sample bridge configuration, while obfs4 requires a
  bridge obtained through Tor Project's bridge channels. The test PC and main
  PC both timed out before receiving any bytes from the single official
  `archive.torproject.org` Expert Bundle URL (20-second connection timeout).
  No archive passed the existing SHA-256/signature gate, no `lyrebird` binary
  was staged, and no bridge transport or Edge Telegram request was launched.
- obfs4 was not attempted because this bounded unattended PoC has no
  user-provided bridge line and must not automate Tor Project CAPTCHA, email,
  Telegram, or bridge harvesting. No mirrors, proxy lists, or alternate binary
  sources were used. The safe result is a technical blocker at verified
  artifact acquisition, not a claim that Snowflake or obfs4 cannot work on the
  test ISP.
- Cleanup after the failed official download passed: the generated
  `bridge-snowflake-*` run directory was removed; `tor.exe`, Edge, `winws.exe`,
  and Tor-owned listeners were zero; global proxy remained disabled without a
  server or PAC. The read-only DNS and firewall state hashes exactly matched
  the pre-test baseline. No product, engine, installer, DNS, proxy, or firewall
  setting changed.
- A future coordinator-run retry is permitted only after a complete official
  Expert Bundle archive, matching detached signature, and official signing key
  are available locally on the test PC. It must rerun the existing provenance
  gate, use exactly the official Snowflake configuration first, bind SOCKS only
  to loopback, and repeat the direct/SOCKS/Edge/leak/cleanup checks. Do not
  attempt obfs4 without a manually supplied official bridge line; do not add
  either transport to the product before a successful remote proof.
- A final official-endpoint audit on 2026-07-29 found no second Tor Project URL
  for the Windows x86_64 Expert Bundle: the official Tor download page points
  its stable Expert Bundle archive and detached signature only to
  `archive.torproject.org`. The separate `dist.torproject.org` links on that
  page are for the Tor source release, not the Expert Bundle. The PoC must not
  guess an unlisted path or use a mirror. This closes automated bootstrap until
  a coordinator supplies the complete archive, matching `.asc`, and official
  key from the listed Tor Project source for the existing verification gate.
- The coordinator later supplied a manually downloaded, official-named Expert
  Bundle archive and matching detached signature on the main PC. The archive
  size was 22,325,312 bytes and its local SHA-256 matched the pinned value
  `6ac067402c7b4a3dc37887ed3754b3914b67fdc220c966190683e9ccf91abf0f`.
  The main PC has no local GPG verifier and must not install or run Tor-related
  software; the detached-signature gate remains intentionally pending on the
  test PC's verified Gpg4win installation.
- On 2026-07-29 the test PC SSH/Tailscale endpoint timed out twice before any
  transfer. No archive, signature, or key bytes were copied; no staging,
  `tor.exe`, `lyrebird`, SOCKS listener, Edge process, or network PoC started.
  Restore test-PC SSH before the next step, then transfer the exact archive and
  `.asc`, obtain the official key, and complete SHA-256 plus `VALIDSIG` before
  any extraction or Snowflake launch.
- SSH was later restored and the coordinator-supplied archive plus `.asc` were
  copied to the test PC. The copied archive again matched the pinned SHA-256,
  but the detached-signature gate stayed fail-closed: the official Tor WKD URL
  timed out and the official documented GnuPG WKD lookup also failed to finish
  within its bounded timeout. No pre-existing Tor signing key was present.
  The transferred archive, signature, temporary GPG homes, and input directory
  were removed without extraction. `tor.exe`, `lyrebird`, Edge, `winws.exe`,
  and Tor listeners were zero afterward; proxy remained disabled and the
  read-only DNS/firewall hashes matched the baseline.
- The only safe unblock is a coordinator-supplied Tor Browser Developers
  public key downloaded manually from the official link in Tor's verification
  documentation. The next test must verify its pinned primary fingerprint
  `EF6E286DDA85EA2A4BA7DE684E2C6E8793298290`, then rerun `gpgv` against the
  existing archive and `.asc` before any extraction or Snowflake launch.
- Next strategy-integration block must prioritize Telegram Web and WhatsApp Web first. Desktop apps are second-stage after Web is confirmed by remote tests.
- Focused Web strategy design and its remote test gate are documented in
  `docs/WEB_STRATEGIES.md`.
- Start with manual `Следующая стратегия` and later `Подобрать автоматически`.
- Health-checks only use DNS resolve, TCP connect, and HTTPS connect.
- No user traffic inspection.
- No infinite switching; use cooldown and attempt limits.
