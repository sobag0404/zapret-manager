# Roadmap

## v1.2 stabilization

- Stabilize engine start/stop lifecycle and tray Exit cleanup.
- Keep `engine-launch.log` useful for failed `winws.exe` starts.
- Verify that disable, emergency disable, and full app exit leave no `winws.exe` runtime processes.
- Keep GitHub Actions green for CI and Windows installer build.

## Future: automatic strategy selection by health-check

Goal: make profile strategy selection safer and easier without inspecting user traffic.

Planned behavior:

- Each profile (`Discord`, `YouTube`, `Telegram`, `WhatsApp`, `Common`) has an ordered strategy list.
- On enable, a profile starts with the first strategy in its list.
- The app runs safe high-level health-checks only:
  - DNS resolve;
  - TCP connect;
  - HTTPS connect to profile domains.
- If a service is unavailable, the supervisor stops the current engine, cleans runtime state, starts the next strategy, and checks again.
- If a strategy works, save it as the active strategy for that profile.
- If all strategies fail, show a clear error and offer manual strategy selection and diagnostics.

Implementation order:

1. Manual buttons: `Следующая стратегия` and `Подобрать автоматически`.
2. Automatic selection during enable.
3. Optional background watchdog only as experimental.

Safety rules:

- Do not read, inspect, or log user traffic.
- Do not switch strategies forever: use cooldown and attempt limits.
- Do not break working Discord/YouTube while trying to fix Telegram/WhatsApp; switching must be profile/group scoped where the engine allows it.
- Background auto-switching requires explicit user consent.
- Implement only after start/stop/exit cleanup and `winws.exe` logging are stable.
