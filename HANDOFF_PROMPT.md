# Zapret Manager Handoff Prompt

Скопируй этот промпт в новый Codex/agent thread на новом устройстве.

```text
Ты продолжаешь разработку проекта Zapret Manager.

Язык общения: русский.
Работай автономно, но не запускай реальный zapret/winws без явного разрешения пользователя, чтобы не конфликтовать с VPN и сетью.

Репозиторий:
- GitHub: https://github.com/sobag0404/zapret-manager
- Ветка: main
- Последний важный commit: 095b402 Fix Flowseal game filter launch
- Проект публичный.

Цель:
Сделать максимально простой рабочий Windows GUI для запуска zapret/Flowseal-подобных стратегий:
- Discord
- YouTube
- Telegram
- Общий режим
- Запуск через GUI
- Выключение через GUI/tray
- Логи
- Диагностика
- Без автозапуска engine после установки

Стек:
- Frontend: Vite + React + TypeScript
- Desktop: Tauri v2
- Backend: Rust
- Installer: Tauri NSIS
- Engine: bundled Flowseal/zapret engine files в engine/local
- Manifest/hash verification обязателен

Что уже сделано:
- GUI есть.
- Installer собирается.
- Реальный Flowseal engine уже лежит в engine/local.
- engine/manifest.json содержит sha256.
- При включении GUI запускает selected Flowseal .bat через UAC.
- GUI не должен напрямую запускать winws.exe без проверки manifest/hash.
- Есть alternative strategies:
  - general
  - alt
  - alt2
  - alt3
  - simple_fake
  - fake_tls_auto
- Есть runtime copy в %LOCALAPPDATA%\ZapretManager\engine-runtime\run-*.

Последние исправленные ошибки:
1. OS 5 Access denied
   Причина: старая runtime-папка могла быть заблокирована elevated winws.exe.
   Исправление: каждый запуск создаёт новую engine-runtime/run-*, старые папки чистятся best-effort.

2. Flowseal strategy запустилась, но winws.exe не найден
   Причина: shim engine/local/service.bat отдавал пустые %GameFilterTCP%/%GameFilterUDP%, из-за этого .bat мог формировать невалидные аргументы.
   Исправление:
   - GameFilterTCP=65535
   - GameFilterUDP=65535
   - обновлён hash service.bat в engine/manifest.json
   - ожидание появления winws.exe увеличено до 8 секунд
   - добавлен engine-launch.log

Очень важно:
- Не переписывай app/tauri/src/service_client.rs через PowerShell Set-Content, иначе можно сломать UTF-8/русские строки.
- Для ручных правок используй apply_patch.
- После изменения любого файла в engine/local обязательно пересчитать sha256 и обновить engine/manifest.json.
- Не запускай сторонние .bat/.exe без понимания, что они делают.
- Не скачивай случайные бинарники.
- Если обновляешь Flowseal/zapret engine, проверяй trusted source, release, hash, manifest.
- Не добавляй телеметрию.
- Не меняй DNS/proxy/firewall без snapshot/revert.

Команды setup на новом Windows:
1. Установить:
   - Git
   - Rust stable x64
   - Node.js LTS
   - pnpm/corepack
   - Visual Studio 2022 Build Tools с C++ Desktop workload
   - WebView2 Runtime
   - NSIS, если Tauri bundler сам не найдёт makensis

2. Клонировать:
   git clone https://github.com/sobag0404/zapret-manager.git
   cd zapret-manager

3. Авторизация GitHub:
   gh auth login
   или настроить новый PAT через GITHUB_TOKEN.
   Не использовать старый засвеченный token.

4. Установка:
   corepack enable
   corepack pnpm install

5. Проверки:
   cargo fmt --all --check
   cargo test --workspace
   corepack pnpm test
   corepack pnpm --dir app/frontend build

6. Сборка installer:
   cd app/tauri
   cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 && cargo tauri build"

7. Готовый installer:
   target/release/bundle/nsis/Zapret Manager_0.1.0_x64-setup.exe

8. Для удобного имени:
   Copy-Item "target\release\bundle\nsis\Zapret Manager_0.1.0_x64-setup.exe" "target\release\bundle\nsis\ZapretManagerSetup.exe" -Force

Если пользователь жалуется, что при включении снова пишет winws.exe не найден:
- Не гадать.
- Найти путь в тексте ошибки к engine-launch.log.
- Прочитать этот лог.
- Проверить:
  - подтвердил ли пользователь UAC;
  - не заблокировал ли WinDivert антивирус;
  - существует ли bin/winws.exe;
  - совпадает ли hash manifest;
  - корректно ли запускается .bat;
  - нет ли старого winws.exe;
  - нет ли конфликта с VPN.

Что доделывать дальше:
- Улучшить tray:
  - закрытие окна должно сворачивать в tray;
  - полный выход только через tray;
  - при полном выходе делать disable_all.
- Сделать полноценную диагностику запуска:
  - показывать engine-launch.log в UI;
  - показывать статус WinDivert;
  - показывать активные PID winws.exe.
- Сделать выбор всех Flowseal стратегий, не только 5.
- Сделать fallback: если general не стартанул, предложить alt/alt2/simple_fake.
- Сделать нормальный service model позже, но сейчас приоритет: простой рабочий Flowseal-like launcher.
- Проверить UI кнопки через Playwright или Tauri window smoke-test.
- Не ломать текущую простую рабочую модель.
```

## Access Notes

- GitHub repo: https://github.com/sobag0404/zapret-manager
- Branch: `main`
- Do not reuse any previously pasted GitHub token.
- Create a new GitHub PAT or use `gh auth login`.
- Local Windows admin rights are needed only for testing real engine launch.
- Do not run the real engine without explicit user approval.
