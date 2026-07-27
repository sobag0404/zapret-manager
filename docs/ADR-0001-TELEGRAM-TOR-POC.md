# ADR-0001: Optional Telegram-Only Routed PoC

Status: Proposed for remote testing only. No production integration.

## Context

Remote evidence on the test ISP shows that Telegram destination TCP 443 is
blocked before TLS/SNI. The existing local DPI-only `winws` path matches the
intended packets but cannot create a route when the destination never returns a
SYN,ACK. Further packet-manipulation experiments are closed for this network.

The proposed alternative is an explicit Telegram-only routed mode using an
unmodified, verified Tor Expert Bundle as a local SOCKS transport. This changes
the product's original promise: Telegram traffic would pass through Tor relays
and an exit relay instead of staying on the direct ISP route. It is not a VPN,
but it is a third-party routed network and must be described that way.

The Tor Project publishes Windows x86_64 and i686 Expert Bundles and describes
them as packages for developers who need to bundle Tor. The Tor Browser support
matrix names Windows 10 and 11, but does not promise support for the exact
Windows 10 18363 build. Compatibility therefore remains a remote PoC gate:
`tor --version`, bootstrap, SOCKS request, and clean shutdown must all pass.

## Decision

Prepare a non-production, coordinator-operated PoC with these boundaries:

- No Tor binary, archive, keyring, bridge, or generated state is committed.
- No network download or Tor process is started by repository tooling.
  Coordinator-invoked verification runs only hash checks, pinned `gpgv`, and a
  pinned extractor to produce an isolated verified staging directory.
- The coordinator manually downloads the stable Windows x86_64 Expert Bundle
  and matching `.asc` from an official Tor Project download link.
- The archive is accepted only after an exact SHA-256 manifest match and a
  `VALIDSIG` from the pinned Tor Browser Developers fingerprint
  `EF6E286DDA85EA2A4BA7DE684E2C6E8793298290`.
- SOCKS listens only on `127.0.0.1` on a per-run high port chosen from an
  OS-assigned free port. `SocksPolicy` rejects non-loopback clients.
- `ControlPort`, `DNSPort`, `TransPort`, `NATDPort`, and `HTTPTunnelPort` are
  disabled. There is no exposed control API or system-wide proxy.
- Each run gets an isolated app-owned data/cache/log directory under
  `%LOCALAPPDATA%\ZapretManager\telegram-tor-poc`.
- `SafeLogging 1` is mandatory. Product diagnostics may retain bootstrap state,
  version, PID, local port, and cleanup status, but never destination addresses,
  traffic content, Telegram account data, cookies, tokens, circuits, relays,
  bridges, usernames, or home-directory paths.
- No DNS, proxy, firewall, adapter, registry, or Telegram private configuration
  is changed globally.

## Web PoC

The first proof uses `curl` with `socks5h://127.0.0.1:<port>` so the hostname is
sent to the SOCKS endpoint instead of being resolved by Windows. Tor's SOCKS
specification supports hostnames and remote resolution; it does not support
SOCKS5 UDP ASSOCIATE.

An optional second proof may launch a separate coordinator-owned Edge/WebView2
profile with a per-process SOCKS argument, QUIC disabled, and host resolution
forced away from the system resolver. This is only an access test. The Tor
Project explicitly discourages routing ordinary browsers through Tor because
they lack Tor Browser's anti-fingerprinting and leak protections. Zapret Manager
must not claim anonymity or privacy equivalence to Tor Browser.

Production Web integration is blocked until a remote test proves:

1. the browser process has no direct Telegram or DNS connection;
2. all target requests fail when the local SOCKS listener is stopped;
3. Disable/Exit terminates the owned Tor process and listener;
4. restart recovery detects and terminates only the app-owned stale process;
5. no persistent browser profile contains Telegram credentials unless the user
   explicitly chose persistence.

## Telegram Desktop

The PoC does not edit Telegram Desktop private configuration files. It does not
inject a proxy, patch the process, or use undocumented command-line switches.

If Telegram Desktop exposes a supported, user-visible SOCKS5 setting, the
coordinator may configure `127.0.0.1:<port>` manually and remove it manually
after the test. A future product may offer a consent/deep-link flow only after
the Telegram-supported format and rollback semantics are independently
verified. Until then, the bounded product UX is:

- automated routed mode: Web only, experimental;
- Desktop: manual proxy consent outside Zapret Manager, or unavailable;
- Disable cannot claim Desktop proxy rollback if Zapret Manager did not set it.

## Lifecycle

A production implementation would place Tor in a Windows Job Object with
kill-on-job-close, retain the exact process handle and creation time, and store
a signed/hashed run record. Disable, tray Exit, emergency disable, crash
recovery, update handoff, and uninstall would:

1. close WebView2/Edge processes created by this mode;
2. terminate only the Tor PID whose executable hash/path and creation time match
   the app-owned run record;
3. verify the SOCKS port is no longer listening;
4. verify no matching app-owned Tor process remains;
5. remove the isolated run directory only after verification;
6. report an error and retain retry state if any invariant fails.

## Distribution and Updates

The PoC does not redistribute Tor. Any future redistribution requires:

- stable-version pinning in a separate Tor manifest;
- official source URL, detached signature, signer fingerprint, archive SHA-256,
  extracted-file hashes, package layout, and supported manager version;
- retained license and third-party notices for every bundled component;
- rollback and revocation handling independent of strategy updates;
- Tor trademark attribution, a link to `torproject.org`, and an explicit
  statement that Zapret Manager is not sponsored by The Tor Project;
- legal review of the complete Expert Bundle's component licenses before
  shipping.

## Consequences

- Telegram access may work where direct destination routing is blocked.
- Latency and reliability depend on Tor relays and ISP reachability to Tor.
- The guard can observe the client IP; the exit can observe destination metadata
  and any plaintext protocol. Telegram TLS remains essential.
- This mode is not compatible with the original "no third-party route" promise;
  explicit opt-in and clear disclosure are mandatory.
- Public proxy lists, free proxy aggregators, remote telemetry, and silent
  fallback to direct traffic are prohibited.

## Official References

- Tor Expert Bundle downloads:
  https://www.torproject.org/download/tor/
- Signature verification and signing fingerprint:
  https://support.torproject.org/tor-browser/getting-started/verifying-tor-browser/
- Windows support baseline:
  https://support.torproject.org/tor-browser/getting-started/installing/
- SOCKS hostname resolution and UDP limitation:
  https://spec.torproject.org/socks-extensions
- Official Expert Bundle/bridge setup:
  https://support.torproject.org/little-t-tor/circumvention/using-bridges/
- Tor Browser warning about ordinary browsers:
  https://support.torproject.org/tor-browser/getting-started/about-tor-browser/
- Trademark and non-affiliation requirements:
  https://www.torproject.org/about/trademark/
