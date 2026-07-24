# Engine Directory

`engine/local/` contains the verified runtime files copied from the Flowseal
`zapret-discord-youtube` release package.

Current source:

- <https://github.com/Flowseal/zapret-discord-youtube/releases/tag/1.9.9c>

Rules:

- bundled `general*.bat` strategies are hash-verified and used as the primary
  launch path;
- do not run upstream installer/update scripts;
- verify every runtime file through `engine/manifest.json`;
- launch binaries only from the bundled, manifest-verified `engine/local/bin`
  directory; runtime under `%LOCALAPPDATA%\ZapretManager\engine-runtime`
  stores only per-run lists and logs;
- stop the child process on normal disable, emergency disable, and tray exit.
