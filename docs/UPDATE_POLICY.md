# Update Policy

Updates can affect the manager, frontend assets, profiles, strategies, service
binary, and engine artifacts. Each update class has different risk.

## Channels

- Stable: default channel for users.
- Preview: opt-in channel for compatibility testing.
- Local: developer or offline bundle source.

## Checks

Every update must pass:

- source allowlist validation;
- version monotonicity checks unless downgrade is explicit;
- checksum or signature verification;
- manifest schema validation;
- compatibility checks;
- rollback readiness checks.

## Apply Order

1. Download and verify candidate artifacts.
2. Snapshot current state.
3. Stop the service if runtime files will change.
4. Apply manager and engine files.
5. Apply profile or strategy data.
6. Restart the service when appropriate.
7. Verify health.
8. Commit the new state or rollback.

## Downgrades

Downgrades require explicit confirmation because profile and state formats may
not be backward compatible.
