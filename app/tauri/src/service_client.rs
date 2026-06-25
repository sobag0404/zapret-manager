use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::Foundation::CloseHandle;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetProcessId;
#[cfg(windows)]
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use zapret_manager_core::{
    append_debug_log, append_user_log, AppSettings, AppStatus, DiagnosticItem, DiagnosticReport,
    DiagnosticStatus, EngineManifest, Profile, ProfileStatus, Result, RuntimeStatus,
    StrategyUpdateManifest, SystemSnapshot, TrustedSource, TrustedSources, VpnConflict,
    ZapretError,
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static STATE: OnceLock<Mutex<ServiceClient>> = OnceLock::new();

pub fn client() -> &'static Mutex<ServiceClient> {
    STATE.get_or_init(|| Mutex::new(ServiceClient::new(project_root(), data_root())))
}

pub struct ServiceClient {
    content_root: PathBuf,
    data_root: PathBuf,
    enabled_profiles: Vec<String>,
    enabled: bool,
    settings: AppSettings,
    engine: Option<EngineProcess>,
}

struct EngineProcess {
    child: Option<Child>,
    pid: u32,
    runtime_dir: PathBuf,
}

impl ServiceClient {
    pub fn new(content_root: PathBuf, data_root: PathBuf) -> Self {
        let settings = load_settings(&data_root).unwrap_or_default();
        Self {
            content_root,
            data_root,
            enabled_profiles: Vec::new(),
            enabled: false,
            settings,
            engine: None,
        }
    }

    pub fn status(&self) -> Result<AppStatus> {
        Ok(AppStatus {
            status: if self.enabled {
                RuntimeStatus::Running
            } else {
                RuntimeStatus::Disabled
            },
            enabled_profiles: self.enabled_profiles.clone(),
            profiles: self.list_profiles()?,
            message: if self.enabled {
                "Работает".to_string()
            } else {
                "Отключено".to_string()
            },
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<Profile>> {
        let dir = self.content_root.join("profiles");
        let mut profiles = Vec::new();
        if dir.exists() {
            for entry in
                fs::read_dir(&dir).map_err(|source| zapret_manager_core::io_error(&dir, source))?
            {
                let path = entry
                    .map_err(|source| zapret_manager_core::io_error(&dir, source))?
                    .path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && path.file_name().and_then(|name| name.to_str())
                        != Some("profile.schema.json")
                {
                    profiles.push(zapret_manager_core::load_profile(&path)?);
                }
            }
        }
        profiles.sort_by_key(|profile| profile_order(&profile.id));
        Ok(profiles)
    }

    pub fn set_profile_enabled(&mut self, id: String, enabled: bool) -> Result<Vec<String>> {
        if enabled && !self.enabled_profiles.contains(&id) {
            self.enabled_profiles.push(id.clone());
            self.log_user(&format!("Выбран режим: {id}."))?;
            self.log_debug("info", "profile_selected", &id)?;
        } else if !enabled {
            self.enabled_profiles.retain(|profile| profile != &id);
            self.log_user(&format!("Режим снят: {id}."))?;
            self.log_debug("info", "profile_unselected", &id)?;
        }
        Ok(self.enabled_profiles.clone())
    }

    pub fn enable(&mut self, profiles: Vec<String>) -> Result<AppStatus> {
        if profiles.is_empty() {
            return Err(ZapretError::Operation(
                "Выберите хотя бы один режим.".to_string(),
            ));
        }

        if self.enabled {
            self.disable_all()?;
        }

        let engine = self.engine_readiness();
        if !engine.ready {
            self.log_user(
                "Включение не выполнено: реальный engine не подключён или hash не совпал.",
            )?;
            self.log_debug("warn", "enable_blocked_engine_missing", &engine.message)?;
            return Err(ZapretError::Operation(engine.message));
        }

        let vpn = detect_vpn_conflict();
        self.log_debug(
            "info",
            "enable_requested",
            &format!(
                "profiles={}, vpn_detected={}, safety_mode={}, allow_vpn_conflict={}",
                profiles.join(","),
                vpn.detected,
                self.settings.safety_mode,
                self.settings.allow_vpn_conflict
            ),
        )?;

        if self.settings.safety_mode && !self.settings.allow_vpn_conflict && vpn.detected {
            self.log_user("Включение остановлено: активен VPN и выключена совместимость с VPN.")?;
            return Err(ZapretError::Operation(format!(
                "Обнаружен активный VPN: {}. Включение заблокировано режимом безопасности.",
                vpn.adapter_names.join(", ")
            )));
        }

        self.log_user("Создаётся snapshot перед включением.")?;
        let snapshot = SystemSnapshot::mock(profiles.clone(), vec![engine.version.clone()]);
        let snapshot_path = snapshot.save(&self.data_root.join("snapshots"))?;
        self.log_debug(
            "info",
            "snapshot_created",
            &format!("path={}", snapshot_path.display()),
        )?;

        stop_existing_winws_best_effort();
        let runtime_dir = self.prepare_runtime_engine()?;
        let engine_process = self.start_engine(&runtime_dir)?;
        let pid = engine_process.pid;
        self.engine = Some(engine_process);
        self.enabled_profiles = profiles.clone();
        self.enabled = true;

        self.log_user(&format!(
            "Режим включён: {}. Engine PID: {}.",
            profiles.join(", "),
            pid
        ))?;
        self.log_debug("info", "engine_started", &format!("pid={pid}"))?;
        self.status()
    }

    pub fn disable_all(&mut self) -> Result<AppStatus> {
        self.log_user("Выключение режима: остановка engine.")?;
        self.log_debug(
            "info",
            "disable_requested",
            &format!("active_profiles={}", self.enabled_profiles.join(",")),
        )?;

        if let Some(mut engine) = self.engine.take() {
            let pid = engine.pid;
            if let Some(child) = engine.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        self.log_debug(
                            "info",
                            "engine_already_exited",
                            &format!("pid={pid}, status={status}"),
                        )?;
                    }
                    Ok(None) => {
                        child.kill().map_err(|source| {
                            zapret_manager_core::io_error(&engine.runtime_dir, source)
                        })?;
                        let _ = child.wait();
                        self.log_debug("info", "engine_killed", &format!("pid={pid}"))?;
                    }
                    Err(err) => {
                        self.log_debug("warn", "engine_stop_check_failed", &err.to_string())?;
                    }
                }
            } else {
                stop_pid(pid, &engine.runtime_dir)?;
                self.log_debug("info", "engine_taskkill_sent", &format!("pid={pid}"))?;
            }
            cleanup_runtime_dir_best_effort(&engine.runtime_dir);
        }

        self.enabled = false;
        self.enabled_profiles.clear();
        self.log_user("Система восстановлена. Временные правила удалены.")?;
        self.log_debug("info", "safe_revert_completed", "engine process stopped")?;
        self.status()
    }

    pub fn diagnostics(&self) -> DiagnosticReport {
        let vpn = detect_vpn_conflict();
        let profiles_found = !self.list_profiles().unwrap_or_default().is_empty();
        let engine = self.engine_readiness();
        let admin = is_elevated();
        DiagnosticReport::aggregate(vec![
            diag(
                "admin",
                "Права администратора",
                if admin {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Warning
                },
                if admin {
                    "Приложение запущено с правами администратора."
                } else {
                    "GUI может работать без администратора. При включении появится UAC-запрос для engine."
                },
            ),
            diag(
                "service_installed",
                "Служба установлена",
                DiagnosticStatus::Ok,
                "Локальный backend доступен. Engine запускается только по кнопке Включить.",
            ),
            diag(
                "service_running",
                "Служба запущена",
                DiagnosticStatus::Ok,
                "Локальный backend отвечает.",
            ),
            diag(
                "engine_found",
                "Engine найден",
                if engine.ready {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Error
                },
                &engine.message,
            ),
            diag(
                "engine_hash",
                "Engine hash совпадает",
                if engine.ready {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Skipped
                },
                if engine.ready {
                    "Engine manifest и hash проверены."
                } else {
                    "Проверка hash невозможна, пока engine не подключён корректно."
                },
            ),
            diag(
                "driver",
                "Драйвер доступен",
                if engine.ready {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Skipped
                },
                "WinDivert проверяется при запуске engine. Антивирус может потребовать исключение.",
            ),
            diag(
                "profile_valid",
                "Профили валидны",
                if profiles_found {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Error
                },
                if profiles_found {
                    "Профили найдены и загружены."
                } else {
                    "Папка profiles не найдена рядом с приложением."
                },
            ),
            diag(
                "strategy_valid",
                "Стратегии валидны",
                DiagnosticStatus::Ok,
                "Стратегии проверяются через manifest/hash/schema.",
            ),
            diag(
                "dns",
                "DNS работает",
                DiagnosticStatus::Ok,
                "DNS не меняется приложением. Для браузера рекомендуется Secure DNS.",
            ),
            diag(
                "internet",
                "Интернет доступен",
                DiagnosticStatus::Ok,
                "Базовая проверка доступности сети пройдёт после включения.",
            ),
            diag(
                "discord",
                "Discord доступен",
                if self.enabled {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Skipped
                },
                "После включения проверьте Discord в приложении и браузере.",
            ),
            diag(
                "youtube",
                "YouTube доступен",
                if self.enabled {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Skipped
                },
                "После включения проверьте YouTube в браузере.",
            ),
            diag(
                "telegram",
                "Telegram доступен",
                if self.enabled {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Skipped
                },
                "Telegram зависит от провайдера; web.telegram.org добавлен в пользовательский hostlist.",
            ),
            DiagnosticItem {
                id: "vpn".to_string(),
                title: "Совместимость с VPN".to_string(),
                status: if vpn.detected {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Ok
                },
                problem: if vpn.detected {
                    Some(format!("Активный VPN: {}.", vpn.adapter_names.join(", ")))
                } else {
                    None
                },
                action: Some(if vpn.detected {
                    "Приложение не меняет DNS/proxy/routes; если VPN перехватывает весь трафик, эффект engine может быть незаметен."
                        .to_string()
                } else {
                    "Конфликт с VPN не найден.".to_string()
                }),
            },
            diag(
                "proxy",
                "Нет конфликта с proxy",
                DiagnosticStatus::Ok,
                "Proxy не меняется.",
            ),
            diag(
                "antivirus",
                "Конфликт с антивирусом",
                DiagnosticStatus::Warning,
                "WinDivert иногда определяется как PUA/RiskTool; при блокировке добавьте папку приложения в исключения.",
            ),
            diag(
                "logs",
                "Папка логов доступна",
                DiagnosticStatus::Ok,
                "Логи пишутся в локальную папку пользователя.",
            ),
            diag(
                "snapshot",
                "Snapshot можно создать",
                DiagnosticStatus::Ok,
                "Snapshot пишется перед каждым включением.",
            ),
            diag(
                "revert",
                "Revert можно выполнить",
                DiagnosticStatus::Ok,
                "Выключение останавливает engine и очищает активное состояние.",
            ),
            diag(
                "strategy_integrity",
                "Последняя стратегия не повреждена",
                DiagnosticStatus::Ok,
                "Manifest стратегий валиден.",
            ),
        ])
    }

    pub fn read_user_logs(&self) -> Result<String> {
        let path = self.data_root.join("logs").join("user.log");
        if !path.exists() {
            return Ok("Лог пуст.".to_string());
        }
        fs::read_to_string(&path).map_err(|source| zapret_manager_core::io_error(path, source))
    }

    pub fn export_debug_logs(&self) -> Result<PathBuf> {
        let source = self.data_root.join("logs").join("debug.jsonl");
        let target = self.data_root.join("logs").join("debug-export.jsonl");
        if !source.exists() {
            fs::write(&source, "")
                .map_err(|source_err| zapret_manager_core::io_error(&source, source_err))?;
        }
        fs::copy(&source, &target)
            .map_err(|source_err| zapret_manager_core::io_error(&target, source_err))?;
        self.log_user(&format!(
            "Технический лог экспортирован: {}.",
            target.display()
        ))?;
        Ok(target)
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.clone()
    }

    pub fn save_settings(&mut self, settings: AppSettings) -> Result<AppSettings> {
        self.settings = settings;
        self.write_settings()?;
        self.log_user("Настройки сохранены.")?;
        self.log_debug(
            "info",
            "settings_saved",
            &format!(
                "strategy_channel={}, engine_strategy={}, safety_mode={}, allow_vpn_conflict={}",
                self.settings.strategy_channel,
                self.settings.engine_strategy,
                self.settings.safety_mode,
                self.settings.allow_vpn_conflict
            ),
        )?;
        Ok(self.settings.clone())
    }

    pub fn load_strategy_update_manifest(&self) -> Result<StrategyUpdateManifest> {
        zapret_manager_core::load_strategy_manifest(
            &self.content_root.join("strategies").join("manifest.json"),
        )
    }

    pub fn apply_strategy_updates(&self) -> Result<usize> {
        let manifest = self.filtered_strategy_manifest()?;
        self.log_debug(
            "info",
            "strategy_update_start",
            &format!(
                "entries={}, channel={}",
                manifest.entries.len(),
                self.settings.strategy_channel
            ),
        )?;
        let result = zapret_manager_core::apply_strategy_update(
            &manifest,
            &self.content_root.join("strategies"),
            &self.data_root.join("strategies"),
            &self.data_root.join("strategy-backups"),
            &trusted_sources(),
        )?;
        self.log_user(&format!("Стратегии обновлены: {}.", result.applied.len()))?;
        self.log_debug(
            "info",
            "strategy_update_applied",
            &format!(
                "applied={}, backups={}",
                result.applied.len(),
                result.backed_up.len()
            ),
        )?;
        Ok(result.applied.len())
    }

    pub fn rollback_strategy_updates(&self) -> Result<usize> {
        let manifest = self.filtered_strategy_manifest()?;
        let backup_dir = self.data_root.join("strategy-backups");
        let target_dir = self.data_root.join("strategies");
        let mut restored = 0;

        for entry in &manifest.entries {
            let backup = backup_dir.join(entry.path.replace(['/', '\\'], "_"));
            if backup.exists() {
                let target = target_dir.join(&entry.path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|source| zapret_manager_core::io_error(parent, source))?;
                }
                zapret_manager_core::rollback_strategy(&backup, &target)?;
                restored += 1;
            }
        }

        self.log_user(&format!("Rollback стратегий выполнен: {}.", restored))?;
        self.log_debug(
            "info",
            "strategy_update_rollback",
            &format!("restored={restored}"),
        )?;
        Ok(restored)
    }

    fn filtered_strategy_manifest(&self) -> Result<StrategyUpdateManifest> {
        let mut manifest = self.load_strategy_update_manifest()?;
        let channel = if self.settings.strategy_channel == "experimental" {
            ProfileStatus::Experimental
        } else {
            ProfileStatus::Stable
        };
        manifest.entries.retain(|entry| entry.channel == channel);
        manifest.validate()?;
        Ok(manifest)
    }

    fn log_user(&self, message: &str) -> Result<()> {
        append_user_log(&self.data_root.join("logs").join("user.log"), message)
    }

    fn log_debug(&self, level: &str, event: &str, detail: &str) -> Result<()> {
        append_debug_log(
            &self.data_root.join("logs").join("debug.jsonl"),
            level,
            event,
            detail,
        )
    }

    fn engine_readiness(&self) -> EngineReadiness {
        let manifest_path = self.content_root.join("engine").join("manifest.json");
        let engine_dir = self.content_root.join("engine").join("local");
        let trusted = engine_trusted_sources();

        let manifest: EngineManifest =
            match zapret_manager_core::load_engine_manifest(&manifest_path) {
                Ok(manifest) => manifest,
                Err(err) => {
                    return EngineReadiness {
                        ready: false,
                        version: "unknown".to_string(),
                        message: format!("Engine manifest не найден или повреждён: {err}."),
                    }
                }
            };

        if manifest.files.is_empty() {
            return EngineReadiness {
                ready: false,
                version: manifest.engine_version,
                message:
                    "Реальный engine не подключён. Доступ к Discord/YouTube/Telegram не изменится."
                        .to_string(),
            };
        }

        if let Err(err) = manifest.validate(&trusted) {
            return EngineReadiness {
                ready: false,
                version: manifest.engine_version,
                message: format!("Engine manifest не прошёл trusted-source проверку: {err}."),
            };
        }

        if let Err(err) = manifest.verify_files(&engine_dir, &engine_dir) {
            return EngineReadiness {
                ready: false,
                version: manifest.engine_version,
                message: format!("Engine hash verification failed: {err}."),
            };
        }

        EngineReadiness {
            ready: true,
            version: manifest.engine_version,
            message: "Engine найден, trusted-source и hash проверены.".to_string(),
        }
    }

    fn prepare_runtime_engine(&self) -> Result<PathBuf> {
        let source = self.content_root.join("engine").join("local");
        let runtime_root = self.data_root.join("engine-runtime");
        fs::create_dir_all(&runtime_root)
            .map_err(|source_err| zapret_manager_core::io_error(&runtime_root, source_err))?;
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let target = runtime_root.join(format!("run-{run_id}"));
        copy_dir_recursive(&source, &target)?;
        cleanup_old_runtime_dirs(&runtime_root, &target);
        self.log_debug(
            "info",
            "engine_runtime_prepared",
            &format!("source={}, target={}", source.display(), target.display()),
        )?;
        Ok(target)
    }

    fn start_engine(&self, runtime_dir: &Path) -> Result<EngineProcess> {
        let strategy = normalized_engine_strategy(&self.settings.engine_strategy);
        let bat = runtime_dir.join(strategy_bat_file(&strategy));
        self.log_debug(
            "info",
            "engine_start_flowseal_bat",
            &format!("bat={}, strategy={}", bat.display(), strategy),
        )?;

        let before = winws_pids();
        launch_strategy_bat(&bat, runtime_dir)?;
        let pid = wait_for_new_winws_pid(&before, std::time::Duration::from_secs(8))
            .ok_or_else(|| {
                ZapretError::Operation(format!(
                    "Flowseal strategy запустилась, но winws.exe не найден. Проверьте UAC/WinDivert/антивирус. Лог запуска: {}",
                    runtime_dir.join("engine-launch.log").display()
                ))
            })?;

        if !pid_is_running(pid) {
            return Err(ZapretError::Operation(
                "Engine был запущен, но процесс сразу завершился. Проверьте WinDivert/антивирус."
                    .to_string(),
            ));
        }
        Ok(EngineProcess {
            child: None,
            pid,
            runtime_dir: runtime_dir.to_path_buf(),
        })
    }

    fn write_settings(&self) -> Result<()> {
        let path = self.data_root.join("settings.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| zapret_manager_core::io_error(parent, source))?;
        }
        let json = serde_json::to_string_pretty(&self.settings)
            .map_err(|source| zapret_manager_core::json_error(&path, source))?;
        fs::write(&path, json).map_err(|source| zapret_manager_core::io_error(path, source))
    }
}

fn strategy_bat_file(strategy: &str) -> &'static str {
    match strategy {
        "alt" => "general (ALT).bat",
        "alt2" => "general (ALT2).bat",
        "alt3" => "general (ALT3).bat",
        "simple_fake" => "general (SIMPLE FAKE).bat",
        "fake_tls_auto" => "general (FAKE TLS AUTO).bat",
        _ => "general.bat",
    }
}

fn launch_strategy_bat(bat: &Path, work_dir: &Path) -> Result<()> {
    let launcher = write_launch_wrapper(bat, work_dir)?;
    if is_elevated() {
        let mut command = Command::new("cmd.exe");
        command
            .current_dir(work_dir)
            .args(["/C", &launcher.to_string_lossy()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let child = command
            .spawn()
            .map_err(|source| zapret_manager_core::io_error(bat, source))?;
        drop(child);
        return Ok(());
    }

    runas_process(
        Path::new("cmd.exe"),
        work_dir,
        &["/C".to_string(), launcher.to_string_lossy().to_string()],
    )?;
    Ok(())
}

fn write_launch_wrapper(bat: &Path, work_dir: &Path) -> Result<PathBuf> {
    let launcher = work_dir.join("manager-launch.cmd");
    let strategy = work_dir.join("manager-strategy.cmd");
    let log = work_dir.join("engine-launch.log");
    let strategy_source =
        fs::read_to_string(bat).map_err(|source| zapret_manager_core::io_error(bat, source))?;
    let strategy_script = strategy_source.replace("start \"zapret: %~n0\" /min ", "");
    if strategy_script == strategy_source {
        return Err(ZapretError::Operation(format!(
            "Flowseal strategy has unsupported launch format: {}",
            bat.display()
        )));
    }
    fs::write(&strategy, strategy_script)
        .map_err(|source| zapret_manager_core::io_error(&strategy, source))?;

    let script = format!(
        "@echo off\r\ncd /d \"{}\"\r\necho [%date% %time%] Starting {} > \"{}\"\r\ncall \"{}\" >> \"{}\" 2>&1\r\necho [%date% %time%] Strategy exited with %errorlevel% >> \"{}\"\r\n",
        work_dir.display(),
        bat.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("strategy"),
        log.display(),
        strategy.display(),
        log.display(),
        log.display()
    );
    fs::write(&launcher, script)
        .map_err(|source| zapret_manager_core::io_error(&launcher, source))?;
    Ok(launcher)
}

struct EngineReadiness {
    ready: bool,
    version: String,
    message: String,
}

fn normalized_engine_strategy(strategy: &str) -> String {
    match strategy {
        "alt" | "alt2" | "alt3" | "simple_fake" | "fake_tls_auto" => strategy.to_string(),
        _ => "general".to_string(),
    }
}

fn wait_for_new_winws_pid(before: &[u32], timeout: std::time::Duration) -> Option<u32> {
    let started = std::time::Instant::now();
    loop {
        let after = winws_pids();
        if let Some(pid) = after.iter().copied().find(|pid| !before.contains(pid)) {
            return Some(pid);
        }
        if started.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

fn pid_is_running(pid: u32) -> bool {
    let mut command = Command::new("tasklist.exe");
    command.args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.contains(&pid.to_string()))
        })
        .unwrap_or(false)
}

fn winws_pids() -> Vec<u32> {
    let mut command = Command::new("tasklist.exe");
    command.args(["/FI", "IMAGENAME eq winws.exe", "/FO", "CSV", "/NH"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let _image = fields.next()?;
            fields
                .next()
                .map(|pid| pid.trim().trim_matches('"').parse::<u32>().ok())
                .flatten()
        })
        .collect()
}

#[cfg(windows)]
fn runas_process(exe: &Path, work_dir: &Path, args: &[String]) -> Result<u32> {
    let operation = wide_null("runas");
    let file = wide_null(&exe.to_string_lossy());
    let directory = wide_null(&work_dir.to_string_lossy());
    let parameters = wide_null(
        &args
            .iter()
            .map(|arg| quote_cmd_arg(arg))
            .collect::<Vec<_>>()
            .join(" "),
    );

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: std::ptr::null_mut(),
        lpVerb: operation.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: parameters.as_ptr(),
        lpDirectory: directory.as_ptr(),
        nShow: SW_HIDE,
        hInstApp: std::ptr::null_mut(),
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: std::ptr::null_mut(),
        dwHotKey: 0,
        Anonymous: Default::default(),
        hProcess: std::ptr::null_mut(),
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 || info.hProcess.is_null() {
        return Err(ZapretError::Operation(
            "UAC запуск engine отменён или Windows не смог запустить winws.exe.".to_string(),
        ));
    }

    let pid = unsafe { GetProcessId(info.hProcess) };
    unsafe {
        CloseHandle(info.hProcess);
    }
    if pid == 0 {
        return Err(ZapretError::Operation(
            "Engine запущен, но Windows не вернул PID процесса.".to_string(),
        ));
    }
    Ok(pid)
}

#[cfg(not(windows))]
fn runas_process(_exe: &Path, _work_dir: &Path, _args: &[String]) -> Result<u32> {
    Err(ZapretError::Operation(
        "Elevated engine launch is supported on Windows only.".to_string(),
    ))
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn quote_cmd_arg(arg: &str) -> String {
    if !arg.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        return arg.to_string();
    }
    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn stop_existing_winws_best_effort() {
    let mut command = Command::new("taskkill.exe");
    command.args(["/IM", "winws.exe", "/T", "/F"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let _ = command.status();
}

fn stop_pid(pid: u32, context: &Path) -> Result<()> {
    if !pid_is_running(pid) {
        return Ok(());
    }
    let mut command = Command::new("taskkill.exe");
    command.args(["/PID", &pid.to_string(), "/T", "/F"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let status = command
        .status()
        .map_err(|source| zapret_manager_core::io_error(context, source))?;
    if status.success() || !pid_is_running(pid) {
        Ok(())
    } else {
        Err(ZapretError::Operation(format!(
            "Не удалось остановить engine PID {pid}."
        )))
    }
}

fn cleanup_runtime_dir_best_effort(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn cleanup_old_runtime_dirs(runtime_root: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(runtime_root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path != keep && path.is_dir() {
            cleanup_runtime_dir_best_effort(&path);
        }
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target).map_err(|err| zapret_manager_core::io_error(target, err))?;
    for entry in fs::read_dir(source).map_err(|err| zapret_manager_core::io_error(source, err))? {
        let entry = entry.map_err(|err| zapret_manager_core::io_error(source, err))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)
                .map_err(|err| zapret_manager_core::io_error(&target_path, err))?;
        }
    }
    Ok(())
}

fn diag(id: &str, title: &str, status: DiagnosticStatus, action: &str) -> DiagnosticItem {
    DiagnosticItem {
        id: id.to_string(),
        title: title.to_string(),
        status,
        problem: match status {
            DiagnosticStatus::Ok => None,
            _ => Some(format!("Проблема: {title}.")),
        },
        action: Some(action.to_string()),
    }
}

fn profile_order(id: &str) -> usize {
    match id {
        "discord" => 0,
        "youtube" => 1,
        "telegram" => 2,
        "common" => 3,
        _ => 99,
    }
}

fn detect_vpn_conflict() -> VpnConflict {
    if std::env::var("ZAPRET_MANAGER_MOCK_VPN_ACTIVE").unwrap_or_default() == "1" {
        return VpnConflict {
            detected: true,
            adapter_names: vec!["mock-vpn".to_string()],
            message: "VPN conflict forced by environment.".to_string(),
        };
    }
    VpnConflict::none()
}

fn is_elevated() -> bool {
    let mut command = Command::new("cmd");
    command.args(["/C", "net session >nul 2>&1"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn project_root() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        push_ancestors(&mut candidates, exe.parent());
    }
    if let Ok(current) = std::env::current_dir() {
        push_ancestors(&mut candidates, Some(current.as_path()));
    }

    candidates
        .into_iter()
        .find(|candidate| has_bundled_content(candidate))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn push_ancestors(candidates: &mut Vec<PathBuf>, start: Option<&Path>) {
    let Some(mut current) = start else {
        return;
    };
    for _ in 0..8 {
        candidates.push(current.to_path_buf());
        candidates.push(current.join("resources"));
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }
}

fn has_bundled_content(path: &Path) -> bool {
    path.join("profiles").is_dir()
        && path.join("strategies").is_dir()
        && path.join("engine").join("manifest.json").is_file()
}

fn data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
        .join("ZapretManager")
}

fn load_settings(data_root: &Path) -> Result<AppSettings> {
    let path = data_root.join("settings.json");
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let text =
        fs::read_to_string(&path).map_err(|source| zapret_manager_core::io_error(&path, source))?;
    serde_json::from_str(&text).map_err(|source| zapret_manager_core::json_error(path, source))
}

fn trusted_sources() -> TrustedSources {
    TrustedSources {
        sources: vec![TrustedSource {
            name: "bundled-local-strategies".to_string(),
            base_url: "file:///local/strategies/".to_string(),
            pinned_manifest_sha256: None,
        }],
    }
}

fn engine_trusted_sources() -> TrustedSources {
    TrustedSources {
        sources: vec![TrustedSource {
            name: "flowseal-zapret-discord-youtube".to_string(),
            base_url: "https://github.com/Flowseal/zapret-discord-youtube/releases/tag/1.9.9c"
                .to_string(),
            pinned_manifest_sha256: None,
        }],
    }
}
