# Web Strategy Audit

## Scope

`telegram_web` and `whatsapp_web` are narrow experimental strategies for the
browser versions only. They do not cover the desktop applications, UDP, MTProto,
or Meta IP ranges.

Each strategy accepts exactly one selected profile. This prevents the general
profile from silently changing its command while a focused Web test is running.

## Inputs

- `list-telegram-web.txt`: `telegram.org`, `t.me`, `web.telegram.org`, and
  `api.telegram.org`.
- `list-whatsapp-web.txt`: `whatsapp.com`, `www.whatsapp.com`,
  `web.whatsapp.com`, `static.whatsapp.net`, and `mmg.whatsapp.net`.

The strategy has one TCP 443 hostlist filter. It does not add `--ipset`, UDP
filters, custom ports, DNS changes, hosts-file changes, proxy changes, or
firewall rules.

## Basis

- Flowseal documents `list-general-user.txt` as the supported extension point
  for domains and `ipset-all.txt` for IP/CIDR lists:
  <https://github.com/Flowseal/zapret-discord-youtube>.
- bol-van documents that `--hostlist` applies to a domain and its subdomains:
  <https://github.com/bol-van/zapret/blob/master/docs/readme.en.md>.
- The desync profile is the already bundled Flowseal `general (ALT).bat` TLS
  profile, restricted to one Web hostlist. No new engine binary or unreviewed
  low-level parameter is introduced.
- Telegram's official CIDR list remains available for a later desktop-specific
  investigation, but is intentionally not used by these Web-only strategies:
  <https://core.telegram.org/resources/cidr.txt>.

## Remote Test Gate

Run each test on the separate Windows PC only, with the fresh v1.2 test
installer and a clean Disable between runs:

1. Select only Telegram, choose `Telegram Web`, enable, then test
   `https://web.telegram.org/a/`, `https://t.me/`, and `https://telegram.org/`.
2. Disable and verify no app-owned `winws.exe` or WinDivert service remains.
3. Select only WhatsApp, choose `WhatsApp Web`, enable, then test
   `https://web.whatsapp.com/` and `https://www.whatsapp.com/`.
4. Export diagnostics after each result. The launch log must show the matching
   `strategy_scope`, `selected_profiles`, and `used_hostlists` value.

Do not claim either strategy works until this remote test is recorded. If Web
does not improve, retain the diagnostics and investigate the ISP-specific DPI
behavior before adding another candidate.
