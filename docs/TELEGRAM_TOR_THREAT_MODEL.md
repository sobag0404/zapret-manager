# Telegram-Only Tor PoC Threat Model

## Overview

This document covers a proposed remote-test-only Telegram routed mode for
Zapret Manager. The existing product is a local Windows application whose
privileged network engine is verified and scoped. This PoC adds a separate,
unprivileged Tor client process and a loopback SOCKS boundary. It does not alter
the production UI, engine, installer, system proxy, DNS, firewall, adapters, or
Telegram files.

The protected assets are Telegram session credentials and content, browser
cookies, the user's direct IP and DNS history, Windows integrity, updater trust,
and the guarantee that Disable/Exit removes all app-owned runtime effects.

## Threat Model, Trust Boundaries, and Assumptions

Trust boundaries:

1. Coordinator to local PoC tooling: all archive, signature, keyring, manifest,
   executable, and directory paths are operator-controlled and must be
   validated before use.
2. Official Tor Project distribution to local archive: the network and download
   path are untrusted; detached signature and pinned fingerprint establish
   provenance, while SHA-256 pins the exact accepted artifact.
3. Telegram client to `127.0.0.1` SOCKS: only an app-owned loopback listener is
   allowed. A pre-existing listener, public bind, or PID/path mismatch fails
   closed.
4. Tor client to Tor relays and exit: relays are third parties. Tor provides a
   routed transport, not trust in exit relays or application-layer encryption.
5. App-owned run directory to other local users/processes: generated state and
   logs may expose usage metadata and require restrictive ACLs in production.
6. Browser/WebView to local network: an ordinary browser may bypass SOCKS via
   DNS, UDP, WebRTC, extensions, or unsupported protocols. No privacy or
   anonymity claim is allowed without direct-socket and DNS-leak proof.

Assumptions:

- Windows itself and the logged-in coordinator are not compromised.
- Telegram uses authenticated encryption/TLS for tested Web traffic.
- The Tor signing fingerprint was obtained from an official, independently
  trusted Tor Project channel.
- The PoC is run only on the separate Windows 10 test PC.

## Attack Surface, Mitigations, and Attacker Stories

| Attack | Impact | Required mitigation |
| --- | --- | --- |
| Replaced Expert Bundle | Arbitrary local code execution | Official URL, detached signature, exact fingerprint, pinned SHA-256, no unsigned fallback |
| Replaced signature verifier | Arbitrary local code execution and false provenance | Absolute `gpgv.exe` path and independently pinned executable SHA-256; never resolve it from `PATH` |
| Archive replaced between checks | Unverified code reaches extraction | Re-hash after `VALIDSIG`, copy into a current-user-only staging directory, hold the staged file against writes, and extract in the same operation |
| Malicious local SOCKS listener wins the port race | Telegram traffic interception | Reject occupied port, start promptly, verify listener PID/path, fail closed |
| SOCKS binds publicly | Open proxy and data exposure | Literal `127.0.0.1`, loopback listener verification, `SocksPolicy` reject all others |
| Browser resolves locally or bypasses SOCKS | DNS/IP leak or direct failure | SOCKS hostname requests, QUIC/UDP disabled for Web test, socket/DNS observation, no silent direct fallback |
| Control port exposed | Circuit manipulation or traffic correlation | `ControlPort 0`; no controller in PoC |
| Logs retain destinations or identity | Privacy leak during export | `SafeLogging 1`, minimal bootstrap-only export, path/address/content redaction |
| Tor process survives Disable/Exit | Continued routed traffic and stale listener | retained process handle, Job Object, scoped PID/path/hash verification, startup stale recovery |
| Broad process cleanup | Kills unrelated Tor software | app-owned root, exact executable hash/path, PID creation time, fail closed on ambiguity |
| Tampered local manifest | Trust bypass | production manifest signed with app update trust root; PoC manifest manually reviewed and kept outside Git |
| Tor network blocked | Availability failure | honest error; optional official bridges only in a separate reviewed phase |
| Exit observes plaintext | Credential/content exposure | require end-to-end encrypted Telegram endpoints; never treat Tor as application encryption |
| Ordinary browser fingerprinting | Correlation/deanonymization | no anonymity claim; prefer Tor Browser for privacy, isolated profile only for bounded access PoC |

Operator-controlled inputs include paths, version choice, local port request,
manual launch, and manual Desktop proxy consent. Attacker-controlled inputs
include archive bytes, network responses, relay/exit behavior, website content,
and any untrusted local process competing for the port or runtime directory.
The config generator cannot reserve its selected port until Tor starts; the
post-launch listener PID/path/bind check is therefore a mandatory PoC gate.

Out of scope: defending against an administrator or compromised OS, providing
anonymity guarantees, hiding Tor use from the ISP, or bypassing enterprise
policy.

## Severity Calibration

Critical:

- accepting or launching an unsigned/mismatched Tor binary;
- exposing an unauthenticated SOCKS/control listener beyond loopback;
- arbitrary process termination or deletion outside the app-owned root.

High:

- direct DNS/network fallback while UI claims Telegram-only Tor routing;
- surviving Tor process after Disable/Exit;
- automatic mutation of Telegram credentials/private configuration without a
  verified rollback.

Medium:

- safe-log failure that exposes destinations, local paths, or relay metadata;
- stale app-owned data directories;
- misleading compatibility or anonymity claims.

Low:

- missing bootstrap progress detail;
- expected Tor latency;
- a PoC-only formatting or documentation defect without runtime effect.

Repository: zapret-manager
Version: 6c19f94d624ca2fe3f970efc28b90296f7e8d5dd
