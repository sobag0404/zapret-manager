# Security

Zapret Manager controls privileged Windows service behavior and network-related
configuration. The security posture is based on minimizing privileged code,
making update provenance explicit, and keeping diagnostics private by default.

## Security Goals

- Only an authorized local user can change runtime state.
- Engine binaries and profile updates are verified before use.
- Privileged operations are narrow, auditable, and reversible.
- Diagnostics do not expose secrets or unnecessary personal data.
- A compromised frontend cannot directly execute arbitrary privileged actions.

## Non-Goals

- Hiding activity from local administrators.
- Providing anonymity guarantees.
- Defending against a fully compromised operating system.
- Bypassing enterprise device management controls.

## Threat Model

| Threat | Impact | Mitigation |
| --- | --- | --- |
| Malicious engine update | Code execution as service user | Pin update sources, verify checksums/signatures, keep previous engine for rollback |
| Compromised frontend process | Unauthorized service actions | Use a narrow local IPC API, validate every request in core/service, require elevation for privileged state changes |
| Tampered profile or strategy | Broken connectivity or unsafe runtime args | Schema validation, allowlisted parameters, compatibility tests before activation |
| Log or diagnostic leakage | Exposure of hostnames, usernames, paths, or tokens | Redaction pipeline, opt-in export, no packet payload capture |
| Partial uninstall or failed update | Broken network state or orphaned service | Transactional apply, service health checks, documented recovery path |
| Supply-chain compromise | Build or release artifact tampering | Locked dependencies, CI test/build gates, release checksums, signed artifacts where available |
| Local low-privilege user abuse | Escalation through service command API | ACL local IPC endpoints, reject shell-like commands, avoid arbitrary file writes |

## Privileged Boundary Requirements

- The service accepts only structured commands.
- The service never accepts raw shell snippets from the frontend.
- File writes are constrained to owned application directories.
- Runtime arguments are derived from validated profile and strategy data.
- In safety mode, enabling profiles is blocked when an active VPN-like adapter
  is detected unless the user explicitly enables VPN compatibility.
- Service start/stop/update operations are logged with timestamps and caller
  identity when available.

## Reporting Vulnerabilities

Do not file public issues for active vulnerabilities. Send a private report to
the maintainers with:

- affected version or commit;
- reproduction steps;
- expected and actual impact;
- logs with secrets removed.
