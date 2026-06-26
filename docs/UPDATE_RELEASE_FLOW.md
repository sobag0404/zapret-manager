# Update and Release Flow

## Goals

- Stable users receive only signed stable releases.
- Test builds are used locally before promotion.
- The app never installs updates silently. The user must click install.
- v1.0 stays as an explicit etalon build and is not overwritten.
- Keep only the current stable installer, the previous stable installer, and explicitly saved etalon builds.

## Signing

- Tauri updater public key is committed in `app/tauri/tauri.conf.json`.
- Private updater key is stored only locally in `.tauri-updater/` and must be copied to GitHub Actions secrets before publishing updater releases.
- Never commit `TAURI_SIGNING_PRIVATE_KEY`, key files, passwords, GitHub tokens, or `.env` files.

Required GitHub secrets for release signing:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## Test Build

1. Build locally or in a draft/prerelease workflow.
2. Name the artifact `ZapretManager vX.Y-test.exe`.
3. Do not attach test builds to the stable updater manifest.
4. Test install, tray behavior, enable/disable, logs, recovery, and update tab.

## Stable Promotion

1. After a test build is confirmed working, create a stable tag: `vX.Y.Z`.
2. Build with updater artifacts enabled and signing secrets.
3. Upload installer, signature, and generated `latest.json` to a public GitHub Release.
4. Stable clients check only `releases/latest/download/latest.json`.

## Rollback

- Keep the previous stable release available.
- If a release is bad, publish a new fixed stable version or point users to the previous installer manually.
- App-level automatic downgrade is disabled by default; strategy rollback remains available inside the app.

## User Safety

- Updates are verified by Tauri updater signature before install.
- Engine/strategy updates remain separate from app updates.
- Logs are not uploaded automatically.
- No telemetry is enabled.
- On app update install, the app first calls `disable_all` to stop active engine state before relaunch.
