# Diagnostics

Diagnostics help explain service state, profile compatibility, engine version,
and recent failures without collecting unnecessary personal data.

## Included Data

- Application version.
- Engine version and checksum.
- Active profile and strategy IDs.
- Service status and startup type.
- Recent manager and service errors.
- Update and rollback metadata.
- Windows version and architecture.

## Excluded Data

- Packet payloads.
- Browser history.
- Credentials, cookies, tokens, or private keys.
- Full environment variable dumps.
- Unrelated file listings.

## Redaction

The diagnostics exporter must redact:

- usernames in paths where possible;
- access tokens and API keys;
- local IP addresses unless explicitly needed;
- hostnames that are not part of the active configuration.

## Export

Diagnostic export is opt-in. The UI should preview the report or summarize
included categories before the user shares it.
