# Engine Policy

The engine is treated as a separately versioned runtime artifact. Zapret Manager
does not blindly execute newly downloaded binaries.

## Acceptance Criteria

An engine candidate is acceptable only when:

- it comes from an approved source;
- its version can be parsed and compared;
- checksum or signature verification passes;
- the package layout matches the expected manifest;
- it passes compatibility checks for the current manager version;
- a rollback copy of the current engine exists or the user explicitly accepts a
  first-install state.

## Version Selection

By default, prefer stable releases over prereleases. Prerelease engines require
an explicit user or developer-channel policy decision.

## Runtime Arguments

The manager must generate runtime arguments from validated profile and strategy
data. Free-form argument entry is a developer feature and must never be enabled
for normal users without clear risk labeling.

## Quarantine

Failed or rejected engine candidates should be quarantined outside the active
runtime path and excluded from automatic retries until metadata changes.
