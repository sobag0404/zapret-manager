# Web Strategy Audit

## Remote Evidence

On the isolated Windows 10 test PC on 2026-07-26, both `telegram_web` and
`whatsapp_web` left Edge HTTPS requests at `ERR_CONNECTION_TIMED_OUT` after
about 21 seconds. `winws.exe` and WinDivert were running during each test; after
Disable both were removed and the runtime directory count was zero.

The engine-off control also timed out. DNS returned Telegram addresses
`149.154.167.99` and `216.239.32.107`, and WhatsApp addresses `31.13.72.52`
and `129.134.30.12`. A five-second TCP probe did not establish TCP 80 or 443
to any of those addresses, while `1.1.1.1:443` and `8.8.8.8:443` connected.

This proves only that TLS/SNI hostlist processing is too late for this test
path. It does not prove whether the upstream cause is a phase-0 DPI filter, a
route/ACL rule, or another TCP-establishment filter.

## Candidates

`telegram_web_phase0` is the sole new candidate. It is an experimental
Telegram Web-only TCP 443 profile with:

- `--ipset=ipset-telegram-phase0.txt` using the complete official Telegram
  CIDR list, including IPv6: <https://core.telegram.org/resources/cidr.txt>.
- `--dpi-desync=syndata,fake,fakedsplit`, ordered as phase 0, phase 1, then
  phase 2 as documented by bol-van.
- No hostlist, UDP filter, custom route, DNS, hosts-file, proxy, firewall, or
  new binary.

bol-van documents that phase-0 modes act during TCP establishment and that a
hostlist cannot select them until a hostname has already entered the IP cache:
<https://github.com/bol-van/zapret/blob/master/docs/quick_start_windows.md>.
`--wsize` is deliberately not used because upstream marks it obsolete.

`telegram_web` and `whatsapp_web` remain packaged for reproducible evidence but
are deprecated and hidden from normal selection. They were hostlist-only and
failed in the recorded matrix.

No WhatsApp phase-0 candidate is added. Meta/WhatsApp does not publish a narrow
static Web CIDR list that can be safely bundled here. A port-wide phase-0 rule
would alter unrelated HTTPS traffic and could break Discord or YouTube.

Zapret targets DPI behavior; its upstream documentation states that a true IP
block is outside its scope. The test matrix must therefore distinguish a
phase-0 improvement from a persistent connection-level block rather than claim
either conclusion prematurely:
<https://github.com/bol-van/zapret/blob/master/docs/readme.en.md>.

## Remote Test Gate

Run only on the separate Windows PC with a clean Disable between cases.

1. Record the engine-off TCP/Edge baseline for `web.telegram.org`.
2. Select only Telegram, choose `Telegram Web: Phase 0`, enable, then repeat
   the same checks for `https://web.telegram.org/` and `https://t.me/`.
3. Confirm the launch log has `strategy_scope=telegram_web_phase0_only`,
   `profile_filters_added=phase0_ipset_strategy`, and
   `covered_by_phase0_ipset=true`.
4. Disable and confirm scoped `winws.exe=0`, app-owned WinDivert is absent, and
   runtime directories equal zero.
5. Run the unchanged WhatsApp engine-off control only. Do not use a broad
   phase-0 rule for WhatsApp.
6. Export diagnostics for every result.

Desktop Telegram and WhatsApp are out of scope until a Web path is confirmed.
