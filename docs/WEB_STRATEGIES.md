# Web Strategy Audit

> Archived research only. Telegram and WhatsApp strategies are not exposed by
> the current Discord/YouTube product manifest or GUI.

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

The static official-Telegram CIDR candidate was tested and did not change the
TCP result. It is now deprecated together with the hostlist-only candidates.

The current diagnostic matrix has two connection-establishment variants for
each Web profile. It is not a broad provider CIDR rule:

- immediately before launch the app resolves a fixed pair of domains:
  `web.telegram.org`/`telegram.org` or
  `web.whatsapp.com`/`www.whatsapp.com`;
- only public, normalized A/AAAA answers are accepted; each domain has a hard
  limit of eight answers and a profile has a hard limit of sixteen total IPs;
- an unexpected, private, loopback, link-local, documentation, empty, or
  over-limit answer fails the diagnostic launch closed;
- the generated `ipset-*-web-runtime.txt` exists only in that run directory,
  is removed by normal Disable cleanup, and is never added to the manifest;
- Windows' standard resolver does not expose DNS TTL through this API. The app
  logs that fact and uses the answers only for the active run.

`Runtime IP / Snd` uses `--dpi-desync=syndata,fake,fakedsplit`.
`Runtime IP / WS` uses `--wssize=1:6`. Both are TCP 443-only, use a hard IP set
before TLS hostname extraction, have no UDP/hostlist/proxy/DNS/firewall/route
changes, and are restricted to exactly one selected profile.

bol-van documents that `syndata` and `wssize` are zero-phase mechanisms, and
that a normal hostlist cannot select them before a hostname is known:
<https://github.com/bol-van/zapret/blob/master/docs/readme.en.md>.
Upstream also warns that `wssize` can slow sites, which is why it is a bounded
manual A/B diagnostic and not an automatic fallback.

`telegram_web` and `whatsapp_web` remain packaged for reproducible evidence but
are deprecated and hidden from normal selection. They were hostlist-only and
failed in the recorded matrix.

No Meta/WhatsApp CIDR is bundled. Runtime answers can still be shared CDN IPs,
so the candidate remains experimental, TCP 443-only, profile-scoped, bounded,
and manual. It is not enabled automatically and must be disabled after each
test before another strategy or profile is used.

Zapret targets DPI behavior; its upstream documentation states that a true IP
block is outside its scope. The test matrix must therefore distinguish a
phase-0 improvement from a persistent connection-level block rather than claim
either conclusion prematurely:
<https://github.com/bol-van/zapret/blob/master/docs/readme.en.md>.

## Remote Test Gate

Run only on the separate Windows PC with a clean Disable between cases.

1. Record the engine-off TCP/Edge baseline and actual resolved addresses for
   `web.telegram.org` and `web.whatsapp.com`.
2. Select only Telegram. Test `Runtime IP / Snd`, Disable, then test `Runtime
   IP / WS`. Repeat the same two cases with WhatsApp only.
3. Confirm each launch log has the matching `strategy_scope`,
   `runtime_dns_domains`, `runtime_dns_accepted_ips`,
   `runtime_dns_ttl=system_resolver_ttl_unavailable`, and
   `runtime_dns_lifetime=run_only`.
4. Disable and confirm scoped `winws.exe=0`, app-owned WinDivert is absent, and
   runtime directories equal zero.
5. If both variants leave raw TCP and Edge identical to the engine-off control,
   classify the result as *no demonstrated DPI improvement*. It is compatible
   with a hard route/IP ACL; this application must not claim it can bypass such
   a block.
6. Export diagnostics for every result.

Desktop Telegram and WhatsApp are out of scope until a Web path is confirmed.
