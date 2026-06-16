# Revert Policy

Zapret Manager must be able to return the machine to the previous known-good
state after failed service changes, engine updates, profile activation, or
uninstall attempts.

## Revertable Operations

- Engine binary replacement.
- Profile activation.
- Strategy activation.
- Service registration changes.
- Service configuration changes.
- Installer repair and uninstall steps.

## Required Metadata

Before applying a change, persist:

- previous engine version and path;
- previous profile and strategy IDs;
- previous service status and startup type;
- files created or replaced by the operation;
- operation timestamp and initiating component;
- verification steps that must pass after apply.

## Rollback Rules

- Rollback must be idempotent.
- Rollback must avoid deleting files not owned by Zapret Manager.
- Rollback must stop the service before replacing runtime files.
- If rollback cannot fully restore state, the UI must expose manual recovery
  steps and diagnostic export.

## User Experience

The user should see a concise reason for the revert, the restored version or
profile, and any residual manual action required.
