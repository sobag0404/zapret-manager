# Engine Policy

The engine is treated as a separately versioned runtime artifact. Zapret Manager
does not blindly execute newly downloaded binaries. The current working engine
source is Flowseal `zapret-discord-youtube` release `1.9.9c`.

## Acceptance Criteria

An engine candidate is acceptable only when:

- it comes from an approved source;
- its version can be parsed and compared;
- checksum or signature verification passes;
- the package layout matches the expected manifest;
- it passes compatibility checks for the current manager version;
- a rollback copy of the current engine exists or the user explicitly accepts a
  first-install state.

The bundled Flowseal archive hash was checked against the GitHub release asset
digest before extracting. Only `bin/*` and `lists/*` are used; upstream scripts
are not executed or packaged into the runtime path.

## Version Selection

By default, prefer stable releases over prereleases. Prerelease engines require
an explicit user or developer-channel policy decision.

## Runtime Arguments

The manager starts `bin/winws.exe` directly with Windows ShellExecute `runas`
from audited Flowseal strategy shapes. The GUI itself does not require
administrator rights; the engine launch requests UAC only when the user presses
`Включить`. Current selectable strategies are `general`, `alt`, `alt2`, `alt3`,
`simple_fake`, and `fake_tls_auto`. Free-form argument entry is a developer
feature and must never be enabled for normal users without clear risk labeling.

## Quarantine

Failed or rejected engine candidates should be quarantined outside the active
runtime path and excluded from automatic retries until metadata changes.
