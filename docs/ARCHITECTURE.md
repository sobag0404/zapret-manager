# Architecture

Zapret Manager is organized around a small privileged surface and a larger
unprivileged management surface.

## Components

- Frontend: desktop UI for profile selection, status, diagnostics, and update
  prompts.
- Core: Rust domain layer that validates desired state, computes service
  actions, and owns policy decisions.
- Windows service: privileged runtime that applies the approved engine and
  profile state.
- Profiles: declarative user-selectable configuration bundles.
- Strategies: compatibility presets that describe how traffic handling should
  be applied.
- Installer: setup, repair, uninstall, service registration, and prerequisites.

## Trust Boundaries

- User session to service: unprivileged UI requests cross into a privileged
  service boundary and must be authenticated locally.
- Project code to bundled engine: the manager treats engine binaries as
  replaceable artifacts with explicit provenance and version metadata.
- Local configuration to runtime state: stored preferences are input, not
  authority; the service must revalidate before applying changes.
- Diagnostics to support channel: generated reports must be redacted before
  leaving the machine.

## Data Flow

1. The user selects a profile and strategy in the frontend.
2. The frontend sends an intent to the Rust core.
3. The core validates compatibility, permissions, and policy constraints.
4. The service applies the approved runtime state.
5. The core records an auditable state transition and rollback metadata.

## Failure Model

Every privileged operation should be modeled as a transaction with:

- preflight checks;
- a bounded apply step;
- verification;
- rollback metadata;
- user-visible recovery instructions.

The manager should prefer a degraded but reversible state over an opaque partial
configuration.
