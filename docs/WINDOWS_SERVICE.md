# Windows Service

The Windows service is the privileged runtime boundary for Zapret Manager.

## Responsibilities

- Start and stop the managed engine process.
- Apply validated runtime configuration.
- Report health and current state to the manager.
- Keep audit logs for privileged operations.
- Participate in rollback and recovery.

## Non-Responsibilities

- Rendering UI.
- Downloading arbitrary files.
- Accepting raw command-line strings from the frontend.
- Modifying unrelated system settings.

## Service Lifecycle

1. Install registers the service with a least-privilege account where practical.
2. Start validates runtime paths and configuration.
3. Health checks confirm that the expected engine process is running.
4. Stop terminates managed processes and releases owned resources.
5. Uninstall removes registration and owned runtime state.

## IPC Contract

The service API should be small and structured:

- query status;
- apply validated state;
- stop managed runtime;
- start managed runtime;
- run diagnostics;
- rollback last operation.

Every command is validated by the service even if the frontend already checked
it.
