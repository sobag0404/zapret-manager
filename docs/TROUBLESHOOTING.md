# Troubleshooting

## Frontend Build Fails

Run:

```powershell
corepack enable
pnpm install --frozen-lockfile
pnpm --dir app/frontend test
pnpm --dir app/frontend build
```

If installation fails, verify that `pnpm-lock.yaml` is committed and
the configured Node.js version matches CI.

## Rust Tests Fail

Run:

```powershell
cargo test --workspace --all-features
```

If the workspace is incomplete, check `Cargo.toml` workspace members before
debugging test code.

## Service Does Not Start

- Confirm the service is installed.
- Check service logs and manager diagnostics.
- Verify engine files exist and match expected checksums.
- Reapply the previous known-good profile.
- Use the recovery flow if startup fails after an update.

## Connectivity Breaks After Profile Change

- Revert to the previous profile.
- Check whether the selected strategy is compatible with the current Windows
  version and engine version.
- Export diagnostics after rollback if the issue persists.

## Update Fails

- Confirm the update source is reachable.
- Verify checksum or signature status.
- Check whether the candidate was quarantined.
- Restore the previous known-good version from rollback metadata.
