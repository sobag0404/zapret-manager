# Installation

This document describes the intended installation flow for Zapret Manager on
Windows.

## Requirements

- Windows 10 or Windows 11.
- Administrator rights for installing or changing the Windows service.
- Network access to configured update sources, unless using an offline bundle.

## Standard Install

1. Download the release artifact from the project release page.
2. Verify the release checksum and signature when published.
3. Run the installer as an administrator.
4. Confirm the service installation prompt.
5. Launch Zapret Manager and select a compatible profile.
6. Apply the profile and confirm the service reaches a healthy state.

## Portable or Developer Install

Portable and developer builds should not silently install a service. They may
run the UI and diagnostics, but service registration must require an explicit
administrator action.

## Upgrade

1. The application downloads or receives a candidate update.
2. The update policy validates source, version, and integrity.
3. The current engine, profile, and service metadata are snapshotted.
4. The update is applied.
5. Health checks run.
6. On failure, the previous known-good state is restored.

## Uninstall

The uninstaller must stop the service, remove service registration, and restore
owned runtime state. It must not delete user-exported diagnostics or unrelated
files.
