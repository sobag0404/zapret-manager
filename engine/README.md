# Engine Directory

`engine/local/` contains the verified runtime files copied from the Flowseal
`zapret-discord-youtube` release package.

Current source:

- <https://github.com/Flowseal/zapret-discord-youtube/releases/tag/1.9.9c>

Rules:

- do not run upstream `.bat`, `.cmd`, `.ps1`, or installer scripts;
- start only `bin/winws.exe`;
- verify every runtime file through `engine/manifest.json`;
- copy files to `%LOCALAPPDATA%\ZapretManager\engine-runtime` before launch;
- stop the child process on normal disable, emergency disable, and tray exit.
