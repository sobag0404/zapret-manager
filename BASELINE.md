# Baseline Build

Эталонная рабочая сборка:

- Date: 2026-06-25
- Git commit: `4de89c6`
- Tag: `baseline-fake-fakedsplit-working`
- Working mode: `fake+fakedsplit`
- Installer source path: `target/release/bundle/nsis/ZapretManagerSetup.exe`
- Local saved copy: `target/release/bundle/nsis/ZapretManagerSetup-baseline-fake-fakedsplit.exe`

Важно:

- Эту сборку считать рабочим rollback-point.
- Перед изменением engine/strategies сверяться с этим состоянием.
- Если новая стратегия сломает запуск или доступность сервисов, откатываться к этому tag.
- Не менять `engine/local` без пересчёта `engine/manifest.json`.
