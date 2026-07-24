use std::ffi::OsStr;
#[cfg(windows)]
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA,
    ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_MARKED_FOR_DELETE, ERROR_SERVICE_NOT_ACTIVE,
    STILL_ACTIVE, S_OK,
};
#[cfg(windows)]
use windows_sys::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, DeleteService, EnumServicesStatusExW, OpenSCManagerW,
    OpenServiceW, QueryServiceConfigW, QueryServiceStatusEx, ENUM_SERVICE_STATUS_PROCESSW,
    QUERY_SERVICE_CONFIGW, SC_ENUM_PROCESS_INFO, SC_HANDLE, SC_MANAGER_CONNECT,
    SC_MANAGER_ENUMERATE_SERVICE, SC_STATUS_PROCESS_INFO, SERVICE_CONTROL_STOP, SERVICE_DRIVER,
    SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATE_ALL, SERVICE_STATUS,
    SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOPPED, SERVICE_STOP_PENDING,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessId, OpenProcess, TerminateProcess, WaitForSingleObject,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};
#[cfg(windows)]
use windows_sys::Win32::UI::Shell::{
    FOLDERID_LocalAppData, SHGetKnownFolderPath, ShellExecuteExW, KF_FLAG_DONT_VERIFY,
    SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
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
#[cfg(windows)]
const WAIT_TIMEOUT: u32 = 0x00000102;
#[cfg(windows)]
const WAIT_FAILED: u32 = 0xFFFF_FFFF;
#[cfg(windows)]
const CLEANUP_WAIT_MS: u32 = 60_000;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_ID: &str = env!("ZAPRET_MANAGER_BUILD_ID");
pub const WINDIVERT_CLEANUP_ARG: &str = "--zapret-manager-cleanup-windivert";

static STATE: OnceLock<Mutex<ServiceClient>> = OnceLock::new();

pub fn client() -> &'static Mutex<ServiceClient> {
    STATE.get_or_init(|| Mutex::new(ServiceClient::new(project_root(), data_root())))
}

pub struct ServiceClient {
    content_root: PathBuf,
    data_root: PathBuf,
    enabled_profiles: Vec<String>,
    enabled: bool,
    cleanup_failed: bool,
    settings: AppSettings,
    engine: Option<EngineProcess>,
}

struct EngineProcess {
    child: Option<Child>,
    #[cfg(windows)]
    process_handle: Option<isize>,
    pid: u32,
    runtime_dir: PathBuf,
    started_at: SystemTime,
}

impl ServiceClient {
    pub fn new(content_root: PathBuf, data_root: PathBuf) -> Self {
        let mut settings = load_settings(&data_root).unwrap_or_default();
        if is_deprecated_strategy(&settings.engine_strategy) {
            settings.engine_strategy = "alt".to_string();
        }
        Self {
            content_root,
            data_root,
            enabled_profiles: Vec::new(),
            enabled: false,
            cleanup_failed: false,
            settings,
            engine: None,
        }
    }

    pub fn status(&self) -> Result<AppStatus> {
        let windivert_report = runtime_windivert_report(&self.data_root.join("engine-runtime"))
            .unwrap_or_else(|err| format!("windivert_check_error={err}"));
        Ok(AppStatus {
            status: runtime_status_from_cleanup_state(
                self.cleanup_failed,
                self.enabled,
                &windivert_report,
            ),
            enabled_profiles: self.enabled_profiles.clone(),
            profiles: self.list_profiles()?,
            message: if self.cleanup_failed {
                "Отключение не завершено".to_string()
            } else if windivert_report.contains("windivert_check_error=") {
                "Отключение не завершено: не удалось проверить WinDivert driver.".to_string()
            } else if !self.enabled && windivert_report_has_app_owned_driver(&windivert_report) {
                "Отключение не завершено: app-owned WinDivert driver всё ещё запущен.".to_string()
            } else if self.enabled {
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
            self.log_user("Включение не выполнено: engine не готов или hash не совпал.")?;
            self.log_debug("warn", "enable_blocked_engine_missing", &engine.message)?;
            return Err(ZapretError::Operation(engine.message));
        }

        let vpn = detect_vpn_conflict();
        self.log_debug(
            "info",
            "enable_requested",
            &format!(
                "profiles={}, vpn_detected={}, safety_mode={}, allow_vpn_conflict={}, strategy={}",
                profiles.join(","),
                vpn.detected,
                self.settings.safety_mode,
                self.settings.allow_vpn_conflict,
                self.settings.engine_strategy
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

        self.cleanup_orphan_runtime_processes("enable_preflight")?;
        self.cleanup_app_owned_windivert("enable_preflight")?;
        let runtime_dir = match self.prepare_runtime_engine() {
            Ok(runtime_dir) => runtime_dir,
            Err(err) => {
                self.log_debug(
                    "error",
                    "engine_runtime_prepare_failed",
                    &format!(
                        "stage=prepare_runtime, strategy={}, error={err}",
                        self.settings.engine_strategy
                    ),
                )?;
                return Err(err);
            }
        };
        let engine_process = match self.start_engine(&runtime_dir, &profiles) {
            Ok(engine_process) => engine_process,
            Err(err) => {
                let mut cleanup_errors = Vec::new();
                if let Err(cleanup_err) =
                    cleanup_orphan_winws_by_runtime(&self.data_root.join("engine-runtime"))
                {
                    cleanup_errors.push(cleanup_err.to_string());
                }
                if let Err(cleanup_err) =
                    cleanup_app_owned_windivert_by_runtime(&self.data_root.join("engine-runtime"))
                {
                    cleanup_errors.push(cleanup_err.to_string());
                }
                if let Err(cleanup_err) =
                    verify_no_app_owned_windivert(&self.data_root.join("engine-runtime"))
                {
                    cleanup_errors.push(cleanup_err.to_string());
                }
                if cleanup_errors.is_empty() {
                    cleanup_runtime_dir_best_effort(&runtime_dir);
                    self.cleanup_failed = false;
                } else {
                    self.cleanup_failed = true;
                }
                self.log_debug(
                    "error",
                    "engine_start_failed",
                    &format!(
                        "stage=start_engine, runtime_dir={}, strategy={}, error={err}, cleanup_errors={}",
                        runtime_dir.display(),
                        self.settings.engine_strategy,
                        cleanup_errors.join(" | ")
                    ),
                )?;
                return if cleanup_errors.is_empty() {
                    Err(err)
                } else {
                    Err(ZapretError::Operation(format!(
                        "{err}; Отключение не завершено: {}",
                        cleanup_errors.join(" | ")
                    )))
                };
            }
        };
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

        let mut cleanup_errors = Vec::new();
        let mut runtime_dirs_to_remove = Vec::new();

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
                        if let Err(err) = child.kill() {
                            cleanup_errors.push(format!("child kill pid={pid}: {err}"));
                            let _ = stop_pid(pid, &engine.runtime_dir);
                        }
                        let _ = child.wait();
                        self.log_debug("info", "engine_killed", &format!("pid={pid}"))?;
                    }
                    Err(err) => {
                        self.log_debug("warn", "engine_stop_check_failed", &err.to_string())?;
                    }
                }
            } else {
                #[cfg(windows)]
                if let Some(handle) = engine.process_handle.take() {
                    if let Err(err) = terminate_process_handle(handle, pid) {
                        cleanup_errors.push(format!("terminate handle pid={pid}: {err}"));
                    } else {
                        self.log_debug(
                            "info",
                            "engine_terminate_handle_sent",
                            &format!("pid={pid}"),
                        )?;
                    }
                } else {
                    if let Err(err) = stop_pid(pid, &engine.runtime_dir) {
                        cleanup_errors.push(format!("terminate pid={pid}: {err}"));
                    }
                    self.log_debug("info", "engine_terminate_sent", &format!("pid={pid}"))?;
                }
                #[cfg(not(windows))]
                {
                    if let Err(err) = stop_pid(pid, &engine.runtime_dir) {
                        cleanup_errors.push(format!("terminate pid={pid}: {err}"));
                    }
                    self.log_debug("info", "engine_terminate_sent", &format!("pid={pid}"))?;
                }
            }
            runtime_dirs_to_remove.push(engine.runtime_dir.clone());
        }

        if let Err(err) = self.cleanup_orphan_runtime_processes("disable_all") {
            cleanup_errors.push(err.to_string());
        }

        if let Err(err) = self.cleanup_app_owned_windivert("disable_all") {
            cleanup_errors.push(err.to_string());
        }

        match verify_no_runtime_winws(&self.data_root.join("engine-runtime")) {
            Ok(report) => {
                self.log_debug(
                    "info",
                    "disable_process_verify",
                    &format!(
                        "runtime_root={}, {report}",
                        self.data_root.join("engine-runtime").display()
                    ),
                )?;
            }
            Err(err) => cleanup_errors.push(err.to_string()),
        }

        match verify_no_app_owned_windivert(&self.data_root.join("engine-runtime")) {
            Ok(report) => {
                self.log_debug(
                    "info",
                    "disable_windivert_verify",
                    &format!(
                        "runtime_root={}, {report}",
                        self.data_root.join("engine-runtime").display()
                    ),
                )?;
            }
            Err(err) => cleanup_errors.push(err.to_string()),
        }

        if cleanup_errors.is_empty() {
            for runtime_dir in runtime_dirs_to_remove {
                cleanup_runtime_dir_best_effort(&runtime_dir);
            }
            let (enabled, profiles, cleanup_failed) =
                disable_state_after_cleanup(self.enabled, &self.enabled_profiles, true);
            self.enabled = enabled;
            self.enabled_profiles = profiles;
            self.cleanup_failed = cleanup_failed;
            self.log_user("Engine остановлен. Активное runtime-состояние очищено. DNS/proxy приложением не менялись.")?;
            self.log_debug("info", "safe_revert_completed", "engine process stopped")?;
        } else {
            let (enabled, profiles, cleanup_failed) =
                disable_state_after_cleanup(self.enabled, &self.enabled_profiles, false);
            self.enabled = enabled;
            self.enabled_profiles = profiles;
            self.cleanup_failed = cleanup_failed;
            self.log_user("Отключение выполнено частично: проверьте диагностику и экспорт логов.")?;
            self.log_debug(
                "error",
                "safe_revert_partial",
                &format!("cleanup_errors={}", cleanup_errors.join(" | ")),
            )?;
            return Err(ZapretError::Operation(format!(
                "Отключение выполнено частично: {}",
                cleanup_errors.join(" | ")
            )));
        }
        self.status()
    }
    pub fn diagnostics(&self) -> DiagnosticReport {
        let vpn = detect_vpn_conflict();
        let profiles_found = !self.list_profiles().unwrap_or_default().is_empty();
        let engine = self.engine_readiness();
        let admin = is_elevated();
        let mut items = vec![
            diag(
                "admin",
                "Права администратора",
                if admin { DiagnosticStatus::Ok } else { DiagnosticStatus::Warning },
                if admin {
                    "Приложение запущено с правами администратора."
                } else {
                    "GUI может работать без администратора. При включении появится UAC-запрос для engine."
                },
            ),
            diag(
                "service_installed",
                "Windows-служба",
                DiagnosticStatus::Skipped,
                "Отдельная Windows-служба в v1.2 не установлена. Управление engine выполняет локальный backend приложения.",
            ),
            diag(
                "service_running",
                "Windows-служба запущена",
                DiagnosticStatus::Skipped,
                "Проверка Windows-службы пропущена: в текущей сборке используется локальный backend, а не отдельная служба.",
            ),
            diag(
                "local_backend",
                "Локальный backend",
                DiagnosticStatus::Ok,
                "Локальный backend отвечает внутри приложения.",
            ),
            diag(
                "engine_found",
                "Engine найден",
                if engine.ready { DiagnosticStatus::Ok } else { DiagnosticStatus::Error },
                &engine.message,
            ),
            diag(
                "engine_hash",
                "Engine hash совпадает",
                if engine.ready { DiagnosticStatus::Ok } else { DiagnosticStatus::Skipped },
                if engine.ready {
                    "Engine manifest и hash проверены."
                } else {
                    "Проверка hash невозможна, пока engine не подключён корректно."
                },
            ),
            diag(
                "driver",
                "Драйвер доступен",
                if engine.ready { DiagnosticStatus::Warning } else { DiagnosticStatus::Skipped },
                "WinDivert проверяется при запуске engine. Антивирус может потребовать исключение.",
            ),
            diag(
                "profile_valid",
                "Профили валидны",
                if profiles_found { DiagnosticStatus::Ok } else { DiagnosticStatus::Error },
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
                "DNS проверка",
                DiagnosticStatus::Skipped,
                "DNS не подтверждён общей диагностикой. Нажмите DNS, чтобы выполнить фактическую проверку резолвинга.",
            ),
            diag(
                "internet",
                "Интернет проверка",
                DiagnosticStatus::Skipped,
                "Доступность интернета не подтверждена общей диагностикой. Запустите проверку доступности.",
            ),
            diag(
                "discord",
                "Discord доступность",
                if self.enabled { DiagnosticStatus::Warning } else { DiagnosticStatus::Skipped },
                "Включённый engine не подтверждает доступность Discord. Запустите проверку доступности и проверьте приложение/браузер.",
            ),
            diag(
                "youtube",
                "YouTube доступность",
                if self.enabled { DiagnosticStatus::Warning } else { DiagnosticStatus::Skipped },
                "Включённый engine не подтверждает доступность YouTube. Запустите проверку доступности и проверьте браузер.",
            ),
            diag(
                "telegram",
                "Telegram доступность",
                if self.enabled { DiagnosticStatus::Warning } else { DiagnosticStatus::Skipped },
                "Доступность Telegram не подтверждена. Запустите Telegram/WhatsApp диагностику и проверьте приложение/браузер.",
            ),
            diag(
                "whatsapp",
                "WhatsApp доступность",
                if self.enabled { DiagnosticStatus::Warning } else { DiagnosticStatus::Skipped },
                "Доступность WhatsApp не подтверждена. Запустите Telegram/WhatsApp диагностику и проверьте приложение/браузер.",
            ),
            DiagnosticItem {
                id: "vpn".to_string(),
                title: "Совместимость с VPN".to_string(),
                status: if vpn.detected { DiagnosticStatus::Warning } else { DiagnosticStatus::Ok },
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
            diag("proxy", "Нет конфликта с proxy", DiagnosticStatus::Ok, "Proxy не меняется."),
            diag(
                "antivirus",
                "Конфликт с антивирусом",
                DiagnosticStatus::Warning,
                "WinDivert иногда определяется как PUA/RiskTool; при блокировке добавьте папку приложения в исключения.",
            ),
            diag("logs", "Папка логов доступна", DiagnosticStatus::Ok, "Логи пишутся в локальную папку пользователя."),
            diag("snapshot", "Snapshot можно создать", DiagnosticStatus::Ok, "Snapshot пишется перед каждым включением."),
            diag("revert", "Revert можно выполнить", DiagnosticStatus::Ok, "Выключение останавливает engine и очищает активное состояние."),
            diag("strategy_integrity", "Последняя стратегия не повреждена", DiagnosticStatus::Ok, "Manifest стратегий валиден."),
        ];
        items.extend(self.runtime_diagnostic_items());
        DiagnosticReport::aggregate(items)
    }

    pub fn dns_diagnostics(&self) -> DiagnosticReport {
        let mut items = Vec::new();
        for (profile, host) in connectivity_targets() {
            let result = check_dns(host);
            items.push(DiagnosticItem {
                id: format!("dns_{profile}_{host}").replace('.', "_"),
                title: format!("DNS {profile}: {host}"),
                status: if result.ok {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Error
                },
                problem: result.problem,
                action: Some(result.action),
            });
        }
        DiagnosticReport::aggregate(items)
    }

    pub fn connectivity_diagnostics(&self) -> DiagnosticReport {
        let mut items = Vec::new();
        items.push(connectivity_item("internet", "one.one.one.one", 443));
        for (profile, host) in connectivity_targets() {
            items.push(connectivity_item(profile, host, 443));
        }
        DiagnosticReport::aggregate(items)
    }

    pub fn messaging_diagnostics(&self) -> DiagnosticReport {
        let mut items = self.runtime_diagnostic_items();
        for (profile, host) in connectivity_targets() {
            if matches!(profile, "telegram" | "whatsapp") {
                let dns = check_dns(host);
                items.push(DiagnosticItem {
                    id: format!("dns_{profile}_{host}").replace('.', "_"),
                    title: format!("DNS {profile}: {host}"),
                    status: if dns.ok {
                        DiagnosticStatus::Ok
                    } else {
                        DiagnosticStatus::Error
                    },
                    problem: dns.problem,
                    action: Some(dns.action),
                });
                items.push(connectivity_item(profile, host, 443));
                items.push(tls_item(profile, host));
            }
        }
        DiagnosticReport::aggregate(items)
    }

    fn runtime_diagnostic_items(&self) -> Vec<DiagnosticItem> {
        let profiles = if self.enabled_profiles.is_empty() {
            "не выбраны".to_string()
        } else {
            self.enabled_profiles.join(", ")
        };
        let latest_log = latest_launch_log(&self.data_root);
        let (engine_status, engine_summary) = self.engine_process_summary();
        let winws_report = runtime_winws_report(&self.data_root.join("engine-runtime"))
            .unwrap_or_else(|err| format!("process_check_error={err}"));
        let windivert_report = runtime_windivert_report(&self.data_root.join("engine-runtime"))
            .unwrap_or_else(|err| format!("windivert_check_error={err}"));
        vec![
            diag(
                "build_identity",
                "Версия сборки",
                if BUILD_ID == "unknown" || BUILD_ID.ends_with("-dirty") {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Ok
                },
                &format!("Zapret Manager {} build {}.", APP_VERSION, BUILD_ID),
            ),
            diag(
                "active_strategy",
                "Активная стратегия",
                DiagnosticStatus::Ok,
                &format!(
                    "Текущая engine strategy: {}.",
                    self.settings.engine_strategy
                ),
            ),
            diag(
                "selected_profiles",
                "Выбранные профили",
                if self.enabled_profiles.is_empty() {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Ok
                },
                &format!("Профили: {profiles}."),
            ),
            diag(
                "launch_log",
                "Лог запуска engine",
                if latest_log.is_some() {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Warning
                },
                &latest_log
                    .as_ref()
                    .map(|path| format!("Последний engine-launch.log: {}.", path.display()))
                    .unwrap_or_else(|| {
                        "engine-launch.log ещё не найден. Нажмите Включить и повторите диагностику."
                            .to_string()
                    }),
            ),
            diag(
                "admin_state",
                "Права администратора",
                if is_elevated() {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Warning
                },
                if is_elevated() {
                    "Процесс запущен с правами администратора."
                } else {
                    "GUI не elevated; при включении будет UAC для winws.exe."
                },
            ),
            diag(
                "engine_process_state",
                "Engine process alive",
                engine_status,
                &engine_summary,
            ),
            diag(
                "winws_runtime_process",
                "Активный winws.exe",
                if winws_report.contains("pid=") {
                    DiagnosticStatus::Ok
                } else {
                    DiagnosticStatus::Warning
                },
                &winws_report,
            ),
            diag(
                "windivert_driver_state",
                "WinDivert driver",
                if windivert_report.contains("windivert_check_error=")
                    || (!self.enabled
                        && (windivert_report_has_app_owned_driver(&windivert_report)
                            || windivert_report_has_running_driver(&windivert_report)))
                {
                    DiagnosticStatus::Error
                } else {
                    DiagnosticStatus::Ok
                },
                &windivert_report,
            ),
        ]
    }

    fn engine_process_summary(&self) -> (DiagnosticStatus, String) {
        let Some(engine) = &self.engine else {
            return (
                DiagnosticStatus::Warning,
                "Engine process is not tracked in current app session.".to_string(),
            );
        };
        let (alive, process_check) = match pid_is_running(engine.pid) {
            Ok(alive) => (alive, "ok".to_string()),
            Err(err) => (true, format!("error={err}")),
        };
        let uptime = engine
            .started_at
            .elapsed()
            .map(|duration| format!("{}s", duration.as_secs()))
            .unwrap_or_else(|_| "unknown".to_string());
        let status = if alive {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Error
        };
        (
            status,
            format!(
                "pid={}, alive={}, process_check={}, uptime={}, runtime_dir={}",
                engine.pid,
                alive,
                process_check,
                uptime,
                sanitize_text(
                    &self.data_root,
                    &self.content_root,
                    &engine.runtime_dir.display().to_string()
                )
            ),
        )
    }

    pub fn read_user_logs(&self) -> Result<String> {
        let path = self.data_root.join("logs").join("user.log");
        if !path.exists() {
            return Ok("Лог пуст.".to_string());
        }
        fs::read_to_string(&path).map_err(|source| zapret_manager_core::io_error(path, source))
    }

    pub fn create_snapshot(&self) -> Result<SystemSnapshot> {
        let snapshot = SystemSnapshot::mock(
            self.enabled_profiles.clone(),
            vec![format!("strategies:1.0.0, build={}", build_identity())],
        );
        snapshot.save(&self.data_root.join("snapshots"))?;
        self.log_user("Snapshot сохранён в локальную папку данных пользователя.")?;
        self.log_debug(
            "info",
            "snapshot_created_manual",
            &format!(
                "path={}, active_profiles={}",
                self.data_root.join("snapshots").display(),
                self.enabled_profiles.join(",")
            ),
        )?;
        Ok(snapshot)
    }

    pub fn restore_snapshot(&mut self) -> Result<AppStatus> {
        self.log_user("Restore snapshot в v1.2 выполняет безопасную часть: остановку engine и очистку runtime state. DNS/proxy приложением не менялись.")?;
        self.disable_all()
    }

    pub fn export_debug_logs(&self) -> Result<PathBuf> {
        let logs_dir = self.data_root.join("logs");
        fs::create_dir_all(&logs_dir)
            .map_err(|source_err| zapret_manager_core::io_error(&logs_dir, source_err))?;
        let target = logs_dir.join("diagnostic-export.txt");
        let debug_log = read_sanitized_log(&self.data_root.join("logs").join("debug.jsonl"), 80);
        let user_log = read_sanitized_log(&self.data_root.join("logs").join("user.log"), 80);
        let launch_log_path = latest_launch_log(&self.data_root);
        let launch_log = launch_log_path
            .as_ref()
            .map(|path| read_sanitized_log(path, 200))
            .unwrap_or_else(|| "engine-launch.log not found".to_string());
        let runtime_report = runtime_winws_report(&self.data_root.join("engine-runtime"))
            .unwrap_or_else(|err| format!("process_check_error={err}"));
        let windivert_report = runtime_windivert_report(&self.data_root.join("engine-runtime"))
            .unwrap_or_else(|err| format!("windivert_check_error={err}"));
        let (_, engine_summary) = self.engine_process_summary();
        let endpoint_checks = diagnostic_report_text(self.connectivity_diagnostics());
        let messaging_checks = diagnostic_report_text(self.messaging_diagnostics());
        let diagnostic_text = format!(
            "Zapret Manager diagnostic export\n\
             version={}\n\
             build_id={}\n\
             enabled={}\n\
             selected_profiles={}\n\
             active_strategy={}\n\
             admin={}\n\
             engine_process_state={}\n\
             winws_runtime_process={}\n\
             windivert_driver_state={}\n\
             latest_launch_log={}\n\n\
             [endpoint checks]\n{}\n\n\
             [telegram whatsapp checks]\n{}\n\n\
             [user.log tail]\n{}\n\n\
             [debug.jsonl tail]\n{}\n\n\
             [engine-launch.log tail]\n{}\n",
            APP_VERSION,
            BUILD_ID,
            self.enabled,
            if self.enabled_profiles.is_empty() {
                "none".to_string()
            } else {
                self.enabled_profiles.join(",")
            },
            self.settings.engine_strategy,
            is_elevated(),
            engine_summary,
            runtime_report,
            windivert_report,
            launch_log_path
                .as_ref()
                .map(|path| sanitize_text(
                    &self.data_root,
                    &self.content_root,
                    &path.display().to_string()
                ))
                .unwrap_or_else(|| "not_found".to_string()),
            endpoint_checks,
            messaging_checks,
            user_log,
            debug_log,
            launch_log
        );
        fs::write(&target, diagnostic_text)
            .map_err(|source_err| zapret_manager_core::io_error(&target, source_err))?;
        self.log_user(&format!(
            "Диагностический пакет экспортирован: {}.",
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
                message: "Реальный engine не подключён. Доступ к Discord/YouTube/Telegram/WhatsApp не изменится."
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
        fs::create_dir_all(&target)
            .map_err(|source_err| zapret_manager_core::io_error(&target, source_err))?;
        copy_dir_recursive(&source.join("lists"), &target.join("lists"))?;
        cleanup_old_runtime_dirs(&runtime_root, &target);
        self.log_debug(
            "info",
            "engine_runtime_prepared",
            &format!(
                "binary_source={}, runtime_lists={}",
                source.display(),
                target.join("lists").display()
            ),
        )?;
        Ok(target)
    }

    fn start_engine(&self, runtime_dir: &Path, profiles: &[String]) -> Result<EngineProcess> {
        let strategy = normalized_engine_strategy(&self.settings.engine_strategy);
        let engine_root = self.content_root.join("engine").join("local");
        let bat = engine_root.join(strategy_bat_file(&strategy));
        let launch = build_winws_launch(&bat, runtime_dir, &engine_root, &strategy, profiles)?;
        self.log_debug(
            "info",
            "engine_start_winws_direct",
            &format!(
                "stage=start_engine, bat={}, strategy={}, profiles={}, exe={}, args={}, hostlists={}, ipsets={}, log={}",
                bat.display(),
                strategy,
                profiles.join(","),
                launch.exe_path.display(),
                launch.args.len(),
                launch.hostlists.join(","),
                launch.ipsets.join(","),
                launch.log_path.display()
            ),
        )?;

        let (pid, mut child, process_handle) = launch_winws(&launch)?;
        std::thread::sleep(std::time::Duration::from_millis(1200));

        if let Some(child_ref) = child.as_mut() {
            if let Some(status) = child_ref
                .try_wait()
                .map_err(|source| zapret_manager_core::io_error(runtime_dir, source))?
            {
                append_launch_log(
                    &launch.log_path,
                    &format!("early_exit=true\nexit_status={status}\n"),
                );
                return Err(ZapretError::Operation(format!(
                    "Engine сразу завершился с кодом {:?}. Build: {}. В engine-launch.log есть preflight и argv_list. Если ошибка повторится, экспортируйте diagnostic-export.txt. Лог запуска: {}",
                    status.code(),
                    build_identity(),
                    launch.log_path.display()
                )));
            }
        }

        let pid_alive = pid_is_running(pid)?;
        if !pid_alive {
            #[cfg(windows)]
            let exit_code = process_handle.and_then(process_handle_exit_code);
            #[cfg(not(windows))]
            let exit_code: Option<u32> = None;
            append_launch_log(
                &launch.log_path,
                &format!("early_exit=true\npid={pid}\nexit_code={exit_code:?}\n"),
            );
            return Err(ZapretError::Operation(format!(
                "Engine был запущен, но процесс сразу завершился. Exit code: {:?}. Build: {}. В engine-launch.log есть preflight и argv_list; экспортируйте diagnostic-export.txt. Проверьте WinDivert/UAC/антивирус. Лог запуска: {}",
                exit_code,
                build_identity(),
                launch.log_path.display()
            )));
        }
        Ok(EngineProcess {
            child,
            #[cfg(windows)]
            process_handle,
            pid,
            runtime_dir: runtime_dir.to_path_buf(),
            started_at: SystemTime::now(),
        })
    }

    fn cleanup_orphan_runtime_processes(&self, stage: &str) -> Result<()> {
        let runtime_root = self.data_root.join("engine-runtime");
        let report = cleanup_orphan_winws_by_runtime(&runtime_root)?;
        if !report.trim().is_empty() {
            self.log_debug(
                "info",
                "orphan_winws_cleanup",
                &format!(
                    "stage={stage}, runtime_root={}, {report}",
                    runtime_root.display()
                ),
            )?;
        }
        Ok(())
    }

    fn cleanup_app_owned_windivert(&self, stage: &str) -> Result<()> {
        let runtime_root = self.data_root.join("engine-runtime");
        let report = cleanup_app_owned_windivert_by_runtime(&runtime_root)?;
        if !report.trim().is_empty() {
            self.log_debug(
                "info",
                "windivert_driver_cleanup",
                &format!(
                    "stage={stage}, runtime_root={}, {report}",
                    runtime_root.display()
                ),
            )?;
        }
        Ok(())
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
        "telegram_web" => "web (TELEGRAM).bat",
        "whatsapp_web" => "web (WHATSAPP).bat",
        "alt" => "general (ALT).bat",
        "alt2" => "general (ALT2).bat",
        "alt3" => "general (ALT3).bat",
        "alt4" => "general (ALT4).bat",
        "alt5" => "general (ALT5).bat",
        "alt6" => "general (ALT6).bat",
        "alt7" => "general (ALT7).bat",
        "alt8" => "general (ALT8).bat",
        "alt9" => "general (ALT9).bat",
        "alt10" => "general (ALT10).bat",
        "alt11" => "general (ALT11).bat",
        "alt12" => "general (ALT12).bat",
        "simple_fake" => "general (SIMPLE FAKE).bat",
        "simple_fake_alt" => "general (SIMPLE FAKE ALT).bat",
        "simple_fake_alt2" => "general (SIMPLE FAKE ALT2).bat",
        "fake_tls_auto" => "general (FAKE TLS AUTO).bat",
        "fake_tls_auto_alt" => "general (FAKE TLS AUTO ALT).bat",
        "fake_tls_auto_alt2" => "general (FAKE TLS AUTO ALT2).bat",
        "fake_tls_auto_alt3" => "general (FAKE TLS AUTO ALT3).bat",
        _ => "general.bat",
    }
}

struct WinwsLaunch {
    exe_path: PathBuf,
    engine_root: PathBuf,
    work_dir: PathBuf,
    args: Vec<String>,
    log_path: PathBuf,
    hostlists: Vec<String>,
    ipsets: Vec<String>,
}

struct LaunchPreflight {
    ok: bool,
    report: String,
    error: Option<String>,
}

fn build_winws_launch(
    bat: &Path,
    runtime_dir: &Path,
    engine_root: &Path,
    strategy: &str,
    selected_profiles: &[String],
) -> Result<WinwsLaunch> {
    validate_strategy_profile_scope(strategy, selected_profiles)?;
    let log = runtime_dir.join("engine-launch.log");
    let strategy_source =
        fs::read_to_string(bat).map_err(|source| zapret_manager_core::io_error(bat, source))?;
    let command_line = extract_winws_command(&strategy_source).ok_or_else(|| {
        ZapretError::Operation(format!("winws.exe command not found in {}", bat.display()))
    })?;
    let bin_dir = engine_root.join("bin");
    let lists_dir = runtime_dir.join("lists");
    let expanded = expand_strategy_vars(&command_line, &bin_dir, &lists_dir);
    let mut parts = split_windows_args(&expanded);
    if parts.is_empty() {
        return Err(ZapretError::Operation(format!(
            "Flowseal strategy has empty winws command: {}",
            bat.display()
        )));
    }

    let exe_path = PathBuf::from(parts.remove(0));
    if exe_path.file_name().and_then(|name| name.to_str()) != Some("winws.exe") {
        return Err(ZapretError::Operation(format!(
            "Flowseal strategy resolved unsupported executable: {}",
            exe_path.display()
        )));
    }
    if !exe_path.is_file() {
        return Err(ZapretError::Operation(format!(
            "winws.exe not found in runtime engine: {}",
            exe_path.display()
        )));
    }
    let hostlists = collect_hostlists(&parts);
    let ipsets = collect_ipsets(&parts);
    let profile_report = profile_launch_report(selected_profiles, strategy, &hostlists, &ipsets);
    let strategy_scope = strategy_scope(strategy);
    let preflight = validate_launch_preflight(&exe_path, &bin_dir, &parts);
    let argv_lines = format_argv_lines(&exe_path, &parts);
    let full_command = format!(
        "{} {}",
        exe_path.display(),
        parts
            .iter()
            .map(|arg| quote_cmd_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    );

    let log_text = format!(
        "Starting winws directly\napp_version={}\nbuild_id={}\nstrategy={}\nnormalized_strategy={}\nstrategy_scope={}\nselected_profiles={}\nprofile_strategy_candidates={}\nprofile_hostlist_coverage={}\nprofile_filters_added={}\nused_hostlists={}\nused_ipsets={}\nadmin={}\nwork_dir={}\nexe={}\nexe_exists={}\nwindivert_dll={}\nwindivert_sys={}\nargv={}\ncommand={}\nstdout_stderr=elevated direct spawn is captured below; UAC runas cannot redirect stdout/stderr\n\n",
        APP_VERSION,
        BUILD_ID,
        bat.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("strategy"),
        strategy,
        strategy_scope,
        normalized_profiles(selected_profiles).join(","),
        profile_report.strategy_candidates,
        profile_report.hostlist_coverage,
        if strategy_scope == "general" {
            "disabled_safe_mode"
        } else {
            "web_hostlist_strategy"
        },
        hostlists.join(","),
        ipsets.join(","),
        is_elevated(),
        bin_dir.display(),
        exe_path.display(),
        exe_path.is_file(),
        bin_dir.join("WinDivert.dll").is_file(),
        bin_dir.join("WinDivert64.sys").is_file(),
        parts.len(),
        full_command
    );
    let log_text = format!(
        "{log_text}preflight_ok={}\npreflight_report_begin\n{}\npreflight_report_end\nargv_list_begin\n{}\nargv_list_end\n\n",
        preflight.ok, preflight.report, argv_lines
    );
    fs::write(&log, log_text).map_err(|source| zapret_manager_core::io_error(&log, source))?;

    if let Some(error) = preflight.error {
        return Err(ZapretError::Operation(format!(
            "Engine preflight failed: {error}. Build: {}. Лог запуска: {}",
            build_identity(),
            log.display()
        )));
    }

    Ok(WinwsLaunch {
        exe_path,
        engine_root: engine_root.to_path_buf(),
        work_dir: bin_dir,
        args: parts,
        log_path: log,
        hostlists,
        ipsets,
    })
}

fn validate_launch_preflight(exe_path: &Path, work_dir: &Path, args: &[String]) -> LaunchPreflight {
    let mut lines = Vec::new();
    let mut failures = Vec::new();

    push_file_check(&mut lines, &mut failures, "exe", exe_path);
    push_file_check(
        &mut lines,
        &mut failures,
        "windivert_dll",
        &work_dir.join("WinDivert.dll"),
    );
    push_file_check(
        &mut lines,
        &mut failures,
        "windivert_sys",
        &work_dir.join("WinDivert64.sys"),
    );
    push_file_check(
        &mut lines,
        &mut failures,
        "cygwin_dll",
        &work_dir.join("cygwin1.dll"),
    );

    for (index, arg) in args.iter().enumerate() {
        if arg.trim().is_empty() {
            failures.push(format!("arg[{index}] is empty"));
            lines.push(format!("arg_empty[{index}]=true"));
        }
        if arg.contains('"') {
            failures.push(format!("arg[{index}] contains raw quote after parsing"));
            lines.push(format!("arg_raw_quote[{index}]=true value={arg}"));
        }
    }

    for (source, path) in referenced_launch_files(args) {
        if path.as_os_str().is_empty() {
            failures.push(format!("{source} has empty path"));
            lines.push(format!("referenced_file_empty source={source}"));
            continue;
        }
        let exists = path.is_file();
        lines.push(format!(
            "referenced_file source={} path={} exists={}",
            source,
            path.display(),
            exists
        ));
        if !exists {
            failures.push(format!("missing {source}: {}", path.display()));
        }
    }

    LaunchPreflight {
        ok: failures.is_empty(),
        report: if lines.is_empty() {
            "no_checks".to_string()
        } else {
            lines.join("\n")
        },
        error: if failures.is_empty() {
            None
        } else {
            Some(failures.join("; "))
        },
    }
}

fn push_file_check(lines: &mut Vec<String>, failures: &mut Vec<String>, name: &str, path: &Path) {
    let exists = path.is_file();
    lines.push(format!("{name}={} exists={exists}", path.display()));
    if !exists {
        failures.push(format!("missing {name}: {}", path.display()));
    }
}

fn referenced_launch_files(args: &[String]) -> Vec<(String, PathBuf)> {
    const PREFIXES: &[&str] = &[
        "--hostlist=",
        "--hostlist-auto=",
        "--hostlist-exclude=",
        "--ipset=",
        "--ipset-ip=",
        "--ipset-exclude=",
        "--dpi-desync-fake-quic=",
        "--dpi-desync-fake-tls=",
        "--dpi-desync-fake-http=",
        "--dpi-desync-fake-discord=",
        "--dpi-desync-fake-stun=",
        "--dpi-desync-fake-unknown-udp=",
        "--dpi-desync-fakedsplit-pattern=",
        "--dpi-desync-split-seqovl-pattern=",
    ];

    let mut files = Vec::new();
    for arg in args {
        for prefix in PREFIXES {
            if let Some(value) = arg.strip_prefix(prefix) {
                let value = value.trim_matches('"');
                if !looks_like_file_path(value) {
                    continue;
                }
                files.push((
                    prefix
                        .trim_end_matches('=')
                        .trim_start_matches("--")
                        .to_string(),
                    PathBuf::from(value),
                ));
            }
        }
    }
    files
}

fn looks_like_file_path(value: &str) -> bool {
    value.contains('\\') || value.contains('/')
}

fn disable_state_after_cleanup(
    enabled: bool,
    profiles: &[String],
    cleanup_ok: bool,
) -> (bool, Vec<String>, bool) {
    if cleanup_ok {
        (false, Vec::new(), false)
    } else if profiles.is_empty() {
        (true, Vec::new(), true)
    } else {
        (enabled || !profiles.is_empty(), profiles.to_vec(), true)
    }
}

fn runtime_status_from_cleanup_state(
    cleanup_failed: bool,
    enabled: bool,
    windivert_report: &str,
) -> RuntimeStatus {
    if cleanup_failed || windivert_report.contains("windivert_check_error=") {
        RuntimeStatus::Error
    } else if enabled {
        RuntimeStatus::Running
    } else if windivert_report_has_app_owned_driver(windivert_report)
        || windivert_report_has_running_driver(windivert_report)
    {
        RuntimeStatus::Error
    } else {
        RuntimeStatus::Disabled
    }
}

fn windivert_report_has_app_owned_driver(report: &str) -> bool {
    report
        .to_ascii_lowercase()
        .contains("app_owned_driver=true")
}

fn windivert_report_has_running_driver(report: &str) -> bool {
    let report = report.to_ascii_lowercase();
    report.contains("state=running") || report.contains("started=true")
}

fn windivert_driver_path_is_app_owned(path_name: &str, runtime_root: &Path) -> bool {
    let driver_path = normalize_windows_path_for_scope(path_name);
    let runtime_root = normalize_windows_path_for_scope(&runtime_root.display().to_string());
    !runtime_root.is_empty()
        && (driver_path == runtime_root
            || driver_path
                .strip_prefix(&runtime_root)
                .is_some_and(|suffix| suffix.starts_with('\\')))
}

fn normalize_windows_path_for_scope(value: &str) -> String {
    let value = value
        .trim()
        .trim_matches('"')
        .trim_start_matches(r"\\?\")
        .trim_start_matches(r"\??\")
        .replace('/', r"\")
        .trim_end_matches('\\')
        .to_ascii_lowercase();
    let mut parts = Vec::new();
    for part in value.split('\\') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if parts.last().is_some_and(|last: &&str| !last.ends_with(':')) {
                parts.pop();
            }
            continue;
        }
        parts.push(part);
    }
    parts.join(r"\")
}

fn format_argv_lines(exe_path: &Path, args: &[String]) -> String {
    let mut lines = vec![format!("arg[0]={}", exe_path.display())];
    lines.extend(
        args.iter()
            .enumerate()
            .map(|(index, arg)| format!("arg[{}]={}", index + 1, arg)),
    );
    lines.join("\n")
}

fn build_identity() -> String {
    format!("{APP_VERSION}+{BUILD_ID}")
}

struct ProfileLaunchReport {
    strategy_candidates: String,
    hostlist_coverage: String,
}

fn profile_launch_report(
    selected_profiles: &[String],
    current_strategy: &str,
    hostlists: &[String],
    ipsets: &[String],
) -> ProfileLaunchReport {
    let profiles = normalized_profiles(selected_profiles);
    let strategy_candidates = profiles
        .iter()
        .map(|profile| {
            format!(
                "{profile}={}",
                profile_strategy_candidates(profile, current_strategy).join("|")
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let lower_hostlists = hostlists
        .iter()
        .map(|hostlist| hostlist.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let lower_ipsets = ipsets
        .iter()
        .map(|ipset| ipset.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let hostlist_coverage = profiles
        .iter()
        .filter(|profile| matches!(profile.as_str(), "telegram" | "whatsapp"))
        .map(|profile| {
            let domains = profile_domains(profile);
            let covered_by_general_user = lower_hostlists
                .iter()
                .any(|hostlist| hostlist.ends_with("list-general-user.txt"));
            let covered_by_profile_list = lower_hostlists
                .iter()
                .any(|hostlist| hostlist.ends_with(&format!("list-{profile}.txt")));
            let covered_by_web_profile_list = lower_hostlists
                .iter()
                .any(|hostlist| hostlist.ends_with(&format!("list-{profile}-web.txt")));
            let covered_by_ipset = profile == "telegram"
                && lower_ipsets
                    .iter()
                    .any(|ipset| ipset.ends_with("ipset-all.txt"));
            format!(
                "{profile}: domains={}, covered_by_list_general_user={covered_by_general_user}, covered_by_profile_list={covered_by_profile_list}, covered_by_web_profile_list={covered_by_web_profile_list}, covered_by_ipset={covered_by_ipset}",
                domains.join("|")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    ProfileLaunchReport {
        strategy_candidates,
        hostlist_coverage: if hostlist_coverage.is_empty() {
            "not_applicable".to_string()
        } else {
            hostlist_coverage
        },
    }
}

fn normalized_profiles(selected_profiles: &[String]) -> Vec<String> {
    let mut profiles = selected_profiles
        .iter()
        .map(|profile| profile.trim().to_ascii_lowercase())
        .filter(|profile| !profile.is_empty())
        .collect::<Vec<_>>();
    if profiles.iter().any(|profile| profile == "common") {
        profiles = vec![
            "discord".to_string(),
            "youtube".to_string(),
            "telegram".to_string(),
            "whatsapp".to_string(),
            "common".to_string(),
        ];
    }
    profiles.sort_by_key(|profile| profile_order(profile));
    profiles.dedup();
    profiles
}

fn profile_strategy_candidates(profile: &str, current_strategy: &str) -> Vec<String> {
    let mut candidates = match profile {
        "telegram" => vec![
            "telegram_web",
            "alt",
            "alt3",
            "simple_fake",
            "general",
            "fake_tls_auto",
        ],
        "whatsapp" => vec![
            "whatsapp_web",
            "alt",
            "alt3",
            "simple_fake",
            "general",
            "fake_tls_auto",
        ],
        "discord" | "youtube" => vec!["alt", "alt3", "simple_fake"],
        "common" => vec!["alt", "alt3", "simple_fake", "general"],
        _ => vec!["general", "alt"],
    }
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let current = normalized_engine_strategy(current_strategy);
    if !candidates.iter().any(|candidate| candidate == &current) {
        candidates.insert(0, current);
    }
    candidates
}

fn validate_strategy_profile_scope(strategy: &str, selected_profiles: &[String]) -> Result<()> {
    let required_profile = match strategy {
        "telegram_web" => Some("telegram"),
        "whatsapp_web" => Some("whatsapp"),
        _ => None,
    };
    let Some(required_profile) = required_profile else {
        return Ok(());
    };

    let profiles = normalized_profiles(selected_profiles)
        .into_iter()
        .filter(|profile| profile != "common")
        .collect::<Vec<_>>();
    if profiles == [required_profile] {
        Ok(())
    } else {
        Err(ZapretError::Operation(format!(
            "Стратегия {} предназначена только для профиля {}. Выключите общий режим и выберите только этот профиль.",
            strategy, required_profile
        )))
    }
}

fn strategy_scope(strategy: &str) -> &'static str {
    match strategy {
        "telegram_web" => "telegram_web_only",
        "whatsapp_web" => "whatsapp_web_only",
        _ => "general",
    }
}

fn profile_domains(profile: &str) -> &'static [&'static str] {
    match profile {
        "telegram" => &[
            "web.telegram.org",
            "t.me",
            "telegram.org",
            "api.telegram.org",
            "desktop.telegram.org",
            "updates.tdesktop.com",
        ],
        "whatsapp" => &[
            "web.whatsapp.com",
            "www.whatsapp.com",
            "whatsapp.com",
            "whatsapp.net",
            "g.whatsapp.net",
            "v.whatsapp.net",
        ],
        "discord" => &["discord.com", "discordapp.com"],
        "youtube" => &["youtube.com", "googlevideo.com"],
        _ => &["example.com"],
    }
}

fn collect_hostlists(args: &[String]) -> Vec<String> {
    let mut hostlists = args
        .iter()
        .filter_map(|arg| {
            arg.strip_prefix("--hostlist=")
                .or_else(|| arg.strip_prefix("--hostlist-auto="))
                .map(|value| value.trim_matches('"').to_string())
        })
        .collect::<Vec<_>>();
    hostlists.sort();
    hostlists.dedup();
    hostlists
}

fn collect_ipsets(args: &[String]) -> Vec<String> {
    let mut ipsets = args
        .iter()
        .filter_map(|arg| {
            arg.strip_prefix("--ipset=")
                .or_else(|| arg.strip_prefix("--ipset-ip="))
                .map(|value| value.trim_matches('"').to_string())
        })
        .collect::<Vec<_>>();
    ipsets.sort();
    ipsets.dedup();
    ipsets
}

fn extract_winws_command(source: &str) -> Option<String> {
    let mut command = String::new();
    let mut collecting = false;

    for line in source.lines() {
        let mut current = line.trim().to_string();
        if !collecting && !current.to_ascii_lowercase().contains("winws.exe") {
            continue;
        }

        if !collecting {
            current = current.replace("start \"zapret: %~n0\" /min ", "");
            collecting = true;
        }

        let continued = current.ends_with('^');
        if continued {
            current.pop();
        }
        command.push_str(current.trim_end());
        command.push(' ');

        if !continued {
            break;
        }
    }

    let command = command.trim().to_string();
    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

fn expand_strategy_vars(command: &str, bin_dir: &Path, lists_dir: &Path) -> String {
    let bin = path_var(bin_dir);
    let lists = path_var(lists_dir);
    unescape_cmd_carets(
        &command
            .replace("%BIN%", &bin)
            .replace("%LISTS%", &lists)
            .replace("%GameFilterTCP%", "65535")
            .replace("%GameFilterUDP%", "65535"),
    )
}

fn path_var(path: &Path) -> String {
    let mut value = path.to_string_lossy().to_string();
    if !value.ends_with('\\') && !value.ends_with('/') {
        value.push('\\');
    }
    value
}

fn split_windows_args(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn unescape_cmd_carets(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '^' {
            if let Some(next) = chars.next() {
                output.push(next);
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn launch_winws(launch: &WinwsLaunch) -> Result<(u32, Option<Child>, Option<isize>)> {
    verify_engine_integrity_for_launch(&launch.engine_root)?;
    append_launch_log(
        &launch.log_path,
        &format!(
            "prelaunch_hash_ok=true\nprelaunch_binary_root={}\n",
            launch.engine_root.display()
        ),
    );

    if is_elevated() {
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&launch.log_path)
            .map_err(|source| zapret_manager_core::io_error(&launch.log_path, source))?;
        let stderr = stdout
            .try_clone()
            .map_err(|source| zapret_manager_core::io_error(&launch.log_path, source))?;

        let mut command = Command::new(&launch.exe_path);
        command
            .current_dir(&launch.work_dir)
            .args(&launch.args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let child = command
            .spawn()
            .map_err(|source| zapret_manager_core::io_error(&launch.exe_path, source))?;
        let pid = child.id();
        append_launch_log(&launch.log_path, &format!("spawn_mode=direct\npid={pid}\n"));
        return Ok((pid, Some(child), None));
    }

    let (pid, process_handle) = runas_process(&launch.exe_path, &launch.work_dir, &launch.args)?;
    append_launch_log(
        &launch.log_path,
        &format!("spawn_mode=runas_uac\npid={pid}\nstdout_stderr=not available with ShellExecute runas\n"),
    );
    Ok((pid, None, Some(process_handle)))
}

fn verify_engine_integrity_for_launch(engine_dir: &Path) -> Result<()> {
    let manifest_path = engine_dir
        .parent()
        .map(|parent| parent.join("manifest.json"))
        .ok_or_else(|| {
            ZapretError::Operation(format!(
                "Engine manifest path cannot be resolved for {}",
                engine_dir.display()
            ))
        })?;
    let manifest = zapret_manager_core::load_engine_manifest(&manifest_path)?;
    manifest.validate(&engine_trusted_sources())?;
    manifest.verify_files(engine_dir, engine_dir)
}

struct EngineReadiness {
    ready: bool,
    version: String,
    message: String,
}

fn normalized_engine_strategy(strategy: &str) -> String {
    match strategy {
        "telegram_web" | "whatsapp_web" | "alt" | "alt2" | "alt3" | "alt4" | "alt5" | "alt6"
        | "alt7" | "alt8" | "alt9" | "alt10" | "alt11" | "alt12" | "simple_fake"
        | "simple_fake_alt" | "simple_fake_alt2" | "fake_tls_auto" | "fake_tls_auto_alt"
        | "fake_tls_auto_alt2" | "fake_tls_auto_alt3" => strategy.to_string(),
        _ => "general".to_string(),
    }
}

fn is_deprecated_strategy(strategy: &str) -> bool {
    matches!(strategy, "alt5" | "alt6")
}

#[cfg(windows)]
fn pid_is_running(pid: u32) -> Result<bool> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(last_win32_error(&format!("OpenProcess({pid})")));
    }
    let mut exit_code = 0;
    let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        Err(last_win32_error(&format!("GetExitCodeProcess({pid})")))
    } else {
        Ok(exit_code == STILL_ACTIVE as u32)
    }
}

#[cfg(not(windows))]
fn pid_is_running(_pid: u32) -> Result<bool> {
    Ok(false)
}

#[cfg(windows)]
fn runas_process(exe: &Path, work_dir: &Path, args: &[String]) -> Result<(u32, isize)> {
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
    if pid == 0 {
        unsafe {
            CloseHandle(info.hProcess);
        }
        return Err(ZapretError::Operation(
            "Engine запущен, но Windows не вернул PID процесса.".to_string(),
        ));
    }
    Ok((pid, info.hProcess as isize))
}

#[cfg(not(windows))]
fn runas_process(_exe: &Path, _work_dir: &Path, _args: &[String]) -> Result<(u32, isize)> {
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
    if !arg.is_empty() && !arg.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        return arg.to_string();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        if ch == '\\' {
            backslashes += 1;
        } else if ch == '"' {
            quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
            quoted.push('"');
            backslashes = 0;
        } else {
            quoted.push_str(&"\\".repeat(backslashes));
            backslashes = 0;
            quoted.push(ch);
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

fn quote_cmdline_args(args: &[&str]) -> String {
    args.iter()
        .map(|arg| quote_cmd_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn stop_pid(pid: u32, context: &Path) -> Result<()> {
    if !pid_is_running(pid)? {
        return Ok(());
    }
    terminate_pid(pid, context)
}

#[cfg(windows)]
fn process_handle_exit_code(handle: isize) -> Option<u32> {
    let mut exit_code = 0;
    let ok = unsafe { GetExitCodeProcess(handle as _, &mut exit_code) };
    if ok == 0 || exit_code == STILL_ACTIVE as u32 {
        None
    } else {
        Some(exit_code)
    }
}

#[cfg(windows)]
fn cleanup_orphan_winws_by_runtime(runtime_root: &Path) -> Result<String> {
    if !runtime_root.exists() {
        return Ok("runtime_root_missing=true".to_string());
    }
    let root_prefix = powershell_single_quote(&runtime_root_command_prefix(runtime_root));
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         $rootPrefix = '{root_prefix}'; \
         function Test-AppRuntimeCommandLine([string]$cmd) {{ \
           if ([string]::IsNullOrWhiteSpace($cmd)) {{ return $false }}; \
           $normalized = $cmd.ToLowerInvariant().Replace([char]47, [char]92); \
           return $normalized.Contains($rootPrefix) \
         }}; \
         $items = Get-CimInstance Win32_Process -Filter \"Name = 'winws.exe'\" -ErrorAction Stop | \
           Select-Object ProcessId,CreationDate,CommandLine; \
         $matches = @(); \
         foreach ($p in @($items)) {{ \
           $cmd = [string]$p.CommandLine; \
           if ([string]::IsNullOrWhiteSpace($cmd)) {{ throw \"winws.exe pid=$($p.ProcessId) has no readable command line\" }}; \
           if (Test-AppRuntimeCommandLine $cmd) {{ $matches += $p }} \
         }}; \
         foreach ($p in @($matches)) {{ \
           $current = Get-CimInstance Win32_Process -Filter \"ProcessId = $($p.ProcessId)\" -ErrorAction Stop; \
           if (-not $current) {{ \"vanished pid=$($p.ProcessId)\"; continue }}; \
           if ($current.CreationDate -ne $p.CreationDate) {{ throw \"winws.exe pid=$($p.ProcessId) identity changed before terminate\" }}; \
           $cmd = [string]$current.CommandLine; \
           if ([string]::IsNullOrWhiteSpace($cmd) -or -not (Test-AppRuntimeCommandLine $cmd)) {{ throw \"winws.exe pid=$($p.ProcessId) failed command line revalidation\" }}; \
           $result = Invoke-CimMethod -InputObject $current -MethodName Terminate -ErrorAction Stop; \
           if ($result.ReturnValue -eq 0) {{ \"terminated pid=$($p.ProcessId)\" }} else {{ throw \"terminate pid=$($p.ProcessId) returned $($result.ReturnValue)\" }} \
         }}"
    );
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", &script]);
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .map_err(|source| zapret_manager_core::io_error(runtime_root, source))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() && stderr.is_empty() {
        let report = if stdout.is_empty() {
            "orphan_count=0".to_string()
        } else {
            stdout
        };
        verify_no_runtime_winws(runtime_root)?;
        Ok(report)
    } else {
        Err(ZapretError::Operation(format!(
            "Scoped orphan cleanup failed: {stderr}"
        )))
    }
}

#[cfg(not(windows))]
fn cleanup_orphan_winws_by_runtime(_runtime_root: &Path) -> Result<String> {
    Ok("windows_only=false".to_string())
}

#[cfg(windows)]
fn verify_no_runtime_winws(runtime_root: &Path) -> Result<String> {
    let report = runtime_winws_report(runtime_root)?;
    if report.contains("pid=") {
        Err(ZapretError::Operation(format!(
            "Scoped winws cleanup incomplete: {report}"
        )))
    } else {
        Ok(report)
    }
}

#[cfg(not(windows))]
fn verify_no_runtime_winws(runtime_root: &Path) -> Result<String> {
    runtime_winws_report(runtime_root)
}

fn powershell_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn runtime_root_command_prefix(runtime_root: &Path) -> String {
    let mut prefix = runtime_root
        .to_string_lossy()
        .replace('/', r"\")
        .trim_end_matches('\\')
        .to_ascii_lowercase();
    prefix.push('\\');
    prefix
}

#[cfg(test)]
fn command_line_references_runtime_root(command_line: &str, runtime_root: &Path) -> bool {
    let command = command_line.replace('/', r"\").to_ascii_lowercase();
    command.contains(&runtime_root_command_prefix(runtime_root))
}

fn append_launch_log(path: &Path, message: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn latest_launch_log(data_root: &Path) -> Option<PathBuf> {
    let runtime_root = data_root.join("engine-runtime");
    let entries = fs::read_dir(runtime_root).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path().join("engine-launch.log"))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn read_sanitized_log(path: &Path, max_lines: usize) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return "not_found".to_string();
    };
    let lines = text.lines().rev().take(max_lines).collect::<Vec<_>>();
    lines
        .into_iter()
        .rev()
        .map(|line| sanitize_text(&data_root(), &project_root(), line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_text(data_root: &Path, content_root: &Path, text: &str) -> String {
    text.replace(
        &data_root.to_string_lossy().to_string(),
        "%LOCALAPPDATA%\\ZapretManager",
    )
    .replace(&content_root.to_string_lossy().to_string(), "%APPDIR%")
}

fn diagnostic_report_text(report: DiagnosticReport) -> String {
    report
        .items
        .into_iter()
        .map(|item| {
            format!(
                "{} [{}] problem={} action={}",
                item.title,
                format!("{:?}", item.status).to_ascii_lowercase(),
                item.problem.unwrap_or_else(|| "none".to_string()),
                item.action.unwrap_or_else(|| "none".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(windows)]
fn runtime_winws_report(runtime_root: &Path) -> Result<String> {
    if !runtime_root.exists() {
        return Ok("runtime_root_missing=true; winws_running=false".to_string());
    }
    let root_prefix = powershell_single_quote(&runtime_root_command_prefix(runtime_root));
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         $rootPrefix = '{root_prefix}'; \
         function Test-AppRuntimeCommandLine([string]$cmd) {{ \
           if ([string]::IsNullOrWhiteSpace($cmd)) {{ return $false }}; \
           $normalized = $cmd.ToLowerInvariant().Replace([char]47, [char]92); \
           return $normalized.Contains($rootPrefix) \
         }}; \
         $items = Get-CimInstance Win32_Process -Filter \"Name = 'winws.exe'\" -ErrorAction Stop | \
           Select-Object ProcessId,CommandLine; \
         $matches = @(); \
         foreach ($p in @($items)) {{ \
           $cmd = [string]$p.CommandLine; \
           if ([string]::IsNullOrWhiteSpace($cmd)) {{ throw \"winws.exe pid=$($p.ProcessId) has no readable command line\" }}; \
           if (Test-AppRuntimeCommandLine $cmd) {{ $matches += $p }} \
         }}; \
         if (-not $matches) {{ 'winws_running=false' }} else {{ \
           foreach ($p in @($matches)) {{ \"pid=$($p.ProcessId); command=$($p.CommandLine)\" }} \
         }}"
    );
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", &script]);
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .map_err(|source| zapret_manager_core::io_error(runtime_root, source))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() && stderr.is_empty() {
        Ok(if stdout.is_empty() {
            "winws_running=false".to_string()
        } else {
            sanitize_text(&data_root(), &project_root(), &stdout)
        })
    } else {
        Err(ZapretError::Operation(format!(
            "Scoped process check failed: {stderr}"
        )))
    }
}

#[cfg(not(windows))]
fn runtime_winws_report(_runtime_root: &Path) -> Result<String> {
    Ok("windows_only=false; winws_running=false".to_string())
}

#[cfg(windows)]
fn cleanup_app_owned_windivert_by_runtime(runtime_root: &Path) -> Result<String> {
    validate_windivert_cleanup_root(runtime_root)?;
    if !is_elevated() {
        runas_self_windivert_cleanup_and_wait(runtime_root)?;
        return Ok(
            "spawn_mode=runas_uac; elevated_cleanup_exit=0; windivert_running=false; app_owned_driver=false"
                .to_string(),
        );
    }

    cleanup_app_owned_windivert_scm(runtime_root)
}

#[cfg(not(windows))]
fn cleanup_app_owned_windivert_by_runtime(_runtime_root: &Path) -> Result<String> {
    Ok("windows_only=false; windivert_running=false; app_owned_driver=false".to_string())
}

#[cfg(windows)]
fn cleanup_app_owned_windivert_scm(runtime_root: &Path) -> Result<String> {
    let initial = enumerate_app_owned_windivert_services(runtime_root)?;
    if initial.is_empty() {
        return Ok("windivert_running=false; app_owned_driver=false".to_string());
    }

    let scm = open_scm(SC_MANAGER_CONNECT)?;
    let mut actions = Vec::new();
    for service in initial {
        let handle = match open_service(
            &scm,
            &service.name,
            SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS | SERVICE_STOP | DELETE_ACCESS,
        )? {
            OpenServiceResult::Open(handle) => handle,
            OpenServiceResult::Missing => {
                actions.push(format!("vanished name={}", service.name));
                continue;
            }
            OpenServiceResult::MarkedForDelete => {
                if service.state == SERVICE_STOPPED {
                    actions.push(format!("marked_for_delete_stopped name={}", service.name));
                    continue;
                }
                return Err(ZapretError::Operation(format!(
                    "Отключение не завершено: WinDivert service {} is marked for delete but still {}; cannot verify cleanup.",
                    service.name,
                    service_state_name(service.state)
                )));
            }
        };
        let binary_path = query_service_binary_path(&handle)?;
        if !validate_app_owned_windivert_driver_path(&binary_path, runtime_root)? {
            return Err(ZapretError::Operation(format!(
                "Отключение не завершено: refusing WinDivert cleanup outside app runtime: name={}, path={}",
                service.name, binary_path
            )));
        }
        stop_service_if_needed(&handle, &service.name)?;
        delete_service_if_present(&handle, &service.name)?;
        actions.push(format!(
            "cleaned name={}; state={}; path={}",
            service.name,
            service_state_name(service.state),
            binary_path
        ));
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        let remaining = enumerate_app_owned_windivert_services(runtime_root)?;
        if remaining.is_empty() {
            actions.push("windivert_running=false; app_owned_driver=false".to_string());
            return Ok(actions.join("\n"));
        }
        if std::time::Instant::now() >= deadline {
            let report = format_windivert_services(&remaining);
            return Err(ZapretError::Operation(format!(
                "Отключение не завершено: app-owned WinDivert driver still present: {report}"
            )));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(windows)]
fn runtime_windivert_report(runtime_root: &Path) -> Result<String> {
    let services = enumerate_app_owned_windivert_services(runtime_root)?;
    if services.is_empty() {
        Ok("windivert_running=false; app_owned_driver=false".to_string())
    } else {
        Ok(format_windivert_services(&services))
    }
}

#[cfg(not(windows))]
fn runtime_windivert_report(_runtime_root: &Path) -> Result<String> {
    Ok("windows_only=false; windivert_running=false; app_owned_driver=false".to_string())
}

#[cfg(windows)]
fn verify_no_app_owned_windivert(runtime_root: &Path) -> Result<String> {
    let report = runtime_windivert_report(runtime_root)?;
    if windivert_report_has_app_owned_driver(&report)
        || windivert_report_has_running_driver(&report)
    {
        Err(ZapretError::Operation(format!(
            "Отключение не завершено: app-owned WinDivert driver still present: {report}"
        )))
    } else {
        Ok(report)
    }
}

#[cfg(not(windows))]
fn verify_no_app_owned_windivert(runtime_root: &Path) -> Result<String> {
    runtime_windivert_report(runtime_root)
}

#[cfg(windows)]
pub fn run_windivert_cleanup_cli(runtime_root: PathBuf) -> i32 {
    match validate_windivert_cleanup_root(&runtime_root)
        .and_then(|_| cleanup_app_owned_windivert_scm(&runtime_root))
        .and_then(|_| verify_no_app_owned_windivert(&runtime_root))
    {
        Ok(_) => 0,
        Err(_) => 2,
    }
}

#[cfg(not(windows))]
pub fn run_windivert_cleanup_cli(_runtime_root: PathBuf) -> i32 {
    0
}

#[cfg(windows)]
fn runas_self_windivert_cleanup_and_wait(runtime_root: &Path) -> Result<u32> {
    let exe_path = std::env::current_exe()
        .map_err(|source| zapret_manager_core::io_error("current_exe", source))?;
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    let operation = wide_null("runas");
    let file = wide_null(&exe_path.to_string_lossy());
    let directory = wide_null(&exe_dir.to_string_lossy());
    let parameters = wide_null(&quote_cmdline_args(&[
        WINDIVERT_CLEANUP_ARG,
        &runtime_root.to_string_lossy(),
    ]));

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
            "Отключение не завершено: UAC cleanup для WinDivert отменён или не запустился."
                .to_string(),
        ));
    }

    let wait = unsafe { WaitForSingleObject(info.hProcess, CLEANUP_WAIT_MS) };
    if wait == WAIT_TIMEOUT {
        unsafe {
            CloseHandle(info.hProcess);
        }
        return Err(ZapretError::Operation(
            "Отключение не завершено: elevated WinDivert cleanup не завершился за 60 секунд."
                .to_string(),
        ));
    }
    if wait == WAIT_FAILED {
        unsafe {
            CloseHandle(info.hProcess);
        }
        return Err(last_win32_error(
            "WaitForSingleObject(elevated WinDivert cleanup)",
        ));
    }

    let mut exit_code = 0;
    let exit_ok = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe {
        CloseHandle(info.hProcess);
    }
    if exit_ok == 0 {
        return Err(ZapretError::Operation(
            "Отключение не завершено: Windows не вернул код elevated cleanup.".to_string(),
        ));
    }
    if exit_code == 0 {
        Ok(exit_code)
    } else {
        Err(ZapretError::Operation(format!(
            "Отключение не завершено: elevated WinDivert cleanup завершился с кодом {exit_code}."
        )))
    }
}

#[cfg(windows)]
const DELETE_ACCESS: u32 = 0x0001_0000;

#[cfg(windows)]
struct ScHandle(SC_HANDLE);

#[cfg(windows)]
impl Drop for ScHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseServiceHandle(self.0);
            }
        }
    }
}

#[cfg(windows)]
enum OpenServiceResult {
    Open(ScHandle),
    Missing,
    MarkedForDelete,
}

#[cfg(windows)]
#[derive(Debug, Clone)]
struct WinDivertServiceReport {
    name: String,
    state: u32,
    binary_path: String,
}

#[cfg(windows)]
fn open_scm(access: u32) -> Result<ScHandle> {
    let handle = unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), access) };
    if handle.is_null() {
        Err(last_win32_error("OpenSCManagerW"))
    } else {
        Ok(ScHandle(handle))
    }
}

#[cfg(windows)]
fn open_service(scm: &ScHandle, name: &str, access: u32) -> Result<OpenServiceResult> {
    if !windivert_service_name_is_safe(name) {
        return Err(ZapretError::Operation(format!(
            "Отключение не завершено: unsafe WinDivert service name: {name}"
        )));
    }
    let service_name = wide_null(name);
    let handle = unsafe { OpenServiceW(scm.0, service_name.as_ptr(), access) };
    if handle.is_null() {
        let code = unsafe { GetLastError() };
        if code == ERROR_SERVICE_DOES_NOT_EXIST {
            Ok(OpenServiceResult::Missing)
        } else if code == ERROR_SERVICE_MARKED_FOR_DELETE {
            Ok(OpenServiceResult::MarkedForDelete)
        } else {
            Err(last_win32_error(&format!("OpenServiceW({name})")))
        }
    } else {
        Ok(OpenServiceResult::Open(ScHandle(handle)))
    }
}

#[cfg(windows)]
fn enumerate_app_owned_windivert_services(
    runtime_root: &Path,
) -> Result<Vec<WinDivertServiceReport>> {
    let scm = open_scm(SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE)?;
    let mut bytes_needed = 0u32;
    let mut services_returned = 0u32;
    let mut resume_handle = 0u32;
    let first = unsafe {
        EnumServicesStatusExW(
            scm.0,
            SC_ENUM_PROCESS_INFO,
            SERVICE_DRIVER,
            SERVICE_STATE_ALL,
            std::ptr::null_mut(),
            0,
            &mut bytes_needed,
            &mut services_returned,
            &mut resume_handle,
            std::ptr::null(),
        )
    };
    if first != 0 && services_returned == 0 {
        return Ok(Vec::new());
    }
    let code = unsafe { GetLastError() };
    if first == 0
        && code != ERROR_MORE_DATA
        && code != ERROR_INSUFFICIENT_BUFFER
        && bytes_needed == 0
    {
        return Err(last_win32_error("EnumServicesStatusExW"));
    }
    if bytes_needed == 0 {
        return Ok(Vec::new());
    }

    let usize_count =
        (bytes_needed as usize + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; usize_count];
    services_returned = 0;
    resume_handle = 0;
    let ok = unsafe {
        EnumServicesStatusExW(
            scm.0,
            SC_ENUM_PROCESS_INFO,
            SERVICE_DRIVER,
            SERVICE_STATE_ALL,
            buffer.as_mut_ptr() as *mut u8,
            (buffer.len() * std::mem::size_of::<usize>()) as u32,
            &mut bytes_needed,
            &mut services_returned,
            &mut resume_handle,
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(last_win32_error("EnumServicesStatusExW"));
    }

    let entries = unsafe {
        std::slice::from_raw_parts(
            buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
            services_returned as usize,
        )
    };
    let mut reports = Vec::new();
    for entry in entries {
        let name = unsafe { wide_ptr_to_string(entry.lpServiceName) };
        if !windivert_service_name_is_safe(&name) {
            continue;
        }
        let enum_state = entry.ServiceStatusProcess.dwCurrentState;
        let handle = match open_service(&scm, &name, SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS)? {
            OpenServiceResult::Open(handle) => handle,
            OpenServiceResult::Missing => continue,
            OpenServiceResult::MarkedForDelete => {
                if enum_state == SERVICE_STOPPED {
                    continue;
                }
                return Err(ZapretError::Operation(format!(
                    "Отключение не завершено: WinDivert service {name} is marked for delete but still {}; cannot verify cleanup.",
                    service_state_name(enum_state)
                )));
            }
        };
        let binary_path = query_service_binary_path(&handle)?;
        if validate_app_owned_windivert_driver_path(&binary_path, runtime_root)? {
            let state = query_service_state(&handle).unwrap_or(enum_state);
            reports.push(WinDivertServiceReport {
                name,
                state,
                binary_path,
            });
        }
    }
    Ok(reports)
}

#[cfg(windows)]
fn query_service_binary_path(handle: &ScHandle) -> Result<String> {
    let mut bytes_needed = 0u32;
    let first =
        unsafe { QueryServiceConfigW(handle.0, std::ptr::null_mut(), 0, &mut bytes_needed) };
    let code = unsafe { GetLastError() };
    if first == 0 && code != ERROR_INSUFFICIENT_BUFFER && code != ERROR_MORE_DATA {
        return Err(last_win32_error("QueryServiceConfigW"));
    }
    if bytes_needed == 0 {
        return Ok(String::new());
    }
    let usize_count =
        (bytes_needed as usize + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; usize_count];
    let config = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
    let ok = unsafe {
        QueryServiceConfigW(
            handle.0,
            config,
            (buffer.len() * std::mem::size_of::<usize>()) as u32,
            &mut bytes_needed,
        )
    };
    if ok == 0 {
        return Err(last_win32_error("QueryServiceConfigW"));
    }
    Ok(unsafe { wide_ptr_to_string((*config).lpBinaryPathName) })
}

#[cfg(windows)]
fn query_service_state(handle: &ScHandle) -> Result<u32> {
    let mut status = SERVICE_STATUS_PROCESS {
        dwServiceType: 0,
        dwCurrentState: 0,
        dwControlsAccepted: 0,
        dwWin32ExitCode: 0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: 0,
        dwProcessId: 0,
        dwServiceFlags: 0,
    };
    let mut bytes_needed = 0u32;
    let ok = unsafe {
        QueryServiceStatusEx(
            handle.0,
            SC_STATUS_PROCESS_INFO,
            &mut status as *mut SERVICE_STATUS_PROCESS as *mut u8,
            std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
            &mut bytes_needed,
        )
    };
    if ok == 0 {
        Err(last_win32_error("QueryServiceStatusEx"))
    } else {
        Ok(status.dwCurrentState)
    }
}

#[cfg(windows)]
fn stop_service_if_needed(handle: &ScHandle, name: &str) -> Result<()> {
    let state = query_service_state(handle)?;
    if state == SERVICE_STOPPED {
        return Ok(());
    }
    let mut status = SERVICE_STATUS {
        dwServiceType: 0,
        dwCurrentState: 0,
        dwControlsAccepted: 0,
        dwWin32ExitCode: 0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: 0,
    };
    let stop_ok = unsafe { ControlService(handle.0, SERVICE_CONTROL_STOP, &mut status) };
    if stop_ok == 0 {
        let code = unsafe { GetLastError() };
        if code != ERROR_SERVICE_NOT_ACTIVE {
            return Err(last_win32_error(&format!("ControlService({name}, STOP)")));
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        let state = query_service_state(handle)?;
        if state == SERVICE_STOPPED {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(ZapretError::Operation(format!(
                "Отключение не завершено: WinDivert service {name} did not stop in time; state={}.",
                service_state_name(state)
            )));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[cfg(windows)]
fn delete_service_if_present(handle: &ScHandle, name: &str) -> Result<()> {
    let ok = unsafe { DeleteService(handle.0) };
    if ok != 0 {
        return Ok(());
    }
    let code = unsafe { GetLastError() };
    if code == ERROR_SERVICE_MARKED_FOR_DELETE || code == ERROR_SERVICE_DOES_NOT_EXIST {
        Ok(())
    } else {
        Err(last_win32_error(&format!("DeleteService({name})")))
    }
}

#[cfg(windows)]
fn format_windivert_services(services: &[WinDivertServiceReport]) -> String {
    if services.is_empty() {
        "windivert_running=false; app_owned_driver=false".to_string()
    } else {
        services
            .iter()
            .map(|service| {
                format!(
                    "app_owned_driver=true; name={}; state={}; started={}; path={}",
                    service.name,
                    service_state_name(service.state),
                    service.state == SERVICE_RUNNING,
                    sanitize_text(&data_root(), &project_root(), &service.binary_path)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(windows)]
fn service_state_name(state: u32) -> &'static str {
    match state {
        SERVICE_STOPPED => "Stopped",
        SERVICE_STOP_PENDING => "StopPending",
        SERVICE_RUNNING => "Running",
        _ => "Other",
    }
}

fn windivert_service_name_is_safe(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.is_empty()
        && lower.starts_with("windivert")
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-')
}

#[cfg(windows)]
fn validate_windivert_cleanup_root(runtime_root: &Path) -> Result<()> {
    let expected = trusted_runtime_root_for_cleanup()?;
    if normalize_windows_path_for_scope(&runtime_root.display().to_string())
        != normalize_windows_path_for_scope(&expected.display().to_string())
    {
        return Err(ZapretError::Operation(format!(
            "Отключение не завершено: refusing WinDivert cleanup for unexpected runtime root: {}",
            runtime_root.display()
        )));
    }
    if expected.exists() && path_has_reparse_component(&expected)? {
        return Err(ZapretError::Operation(format!(
            "Отключение не завершено: refusing WinDivert cleanup through a reparse point: {}",
            expected.display()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_app_owned_windivert_driver_path(path_name: &str, runtime_root: &Path) -> Result<bool> {
    validate_windivert_cleanup_root(runtime_root)?;
    let driver_path = cleaned_windows_driver_path(path_name);
    let Some(root) = app_owned_windivert_roots(runtime_root)?
        .into_iter()
        .find(|root| windivert_driver_path_is_app_owned(path_name, root))
    else {
        return Ok(false);
    };
    if path_has_reparse_component(&root)? || path_has_reparse_component(&driver_path)? {
        return Err(ZapretError::Operation(format!(
            "Отключение не завершено: refusing WinDivert cleanup through a reparse point: {}",
            driver_path.display()
        )));
    }
    if !driver_path.exists() {
        return Ok(true);
    }
    let root =
        fs::canonicalize(&root).map_err(|source| zapret_manager_core::io_error(&root, source))?;
    let driver = fs::canonicalize(&driver_path)
        .map_err(|source| zapret_manager_core::io_error(&driver_path, source))?;
    if normalized_path_is_under(&driver, &root) {
        Ok(true)
    } else {
        Err(ZapretError::Operation(format!(
            "Отключение не завершено: refusing WinDivert cleanup outside app runtime after canonical check: {}",
            driver.display()
        )))
    }
}

#[cfg(windows)]
fn app_owned_windivert_roots(runtime_root: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = vec![runtime_root.to_path_buf()];
    let bundled_engine_root = project_root().join("engine").join("local");
    if bundled_engine_root.is_dir() {
        roots.push(bundled_engine_root);
    }
    Ok(roots)
}

#[cfg(windows)]
fn trusted_runtime_root_for_cleanup() -> Result<PathBuf> {
    known_local_app_data_dir()
        .map(|path| path.join("ZapretManager").join("engine-runtime"))
        .ok_or_else(|| {
            ZapretError::Operation(
                "Отключение не завершено: Windows did not return LocalAppData known folder."
                    .to_string(),
            )
        })
}

#[cfg(windows)]
fn cleaned_windows_driver_path(value: &str) -> PathBuf {
    PathBuf::from(
        value
            .trim()
            .trim_matches('"')
            .trim_start_matches(r"\\?\")
            .trim_start_matches(r"\??\")
            .replace('/', r"\"),
    )
}

#[cfg(windows)]
fn normalized_path_is_under(path: &Path, root: &Path) -> bool {
    let path = normalize_windows_path_for_scope(&path.display().to_string());
    let root = normalize_windows_path_for_scope(&root.display().to_string());
    !root.is_empty()
        && (path == root
            || path
                .strip_prefix(&root)
                .is_some_and(|suffix| suffix.starts_with('\\')))
}

#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT_LOCAL: u32 = 0x0000_0400;

#[cfg(windows)]
fn path_has_reparse_component(path: &Path) -> Result<bool> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if !current.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(&current)
            .map_err(|source| zapret_manager_core::io_error(&current, source))?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT_LOCAL != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn last_win32_error(context: &str) -> ZapretError {
    ZapretError::Operation(format!("{context} failed with Win32 error {}", unsafe {
        GetLastError()
    }))
}

#[cfg(windows)]
unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    OsString::from_wide(std::slice::from_raw_parts(ptr, len))
        .to_string_lossy()
        .into_owned()
}

struct NetworkCheck {
    ok: bool,
    problem: Option<String>,
    action: String,
}

fn connectivity_targets() -> Vec<(&'static str, &'static str)> {
    vec![
        ("discord", "discord.com"),
        ("youtube", "youtube.com"),
        ("telegram", "web.telegram.org"),
        ("telegram", "t.me"),
        ("telegram", "api.telegram.org"),
        ("whatsapp", "web.whatsapp.com"),
        ("whatsapp", "www.whatsapp.com"),
        ("whatsapp", "static.whatsapp.net"),
        ("whatsapp", "mmg.whatsapp.net"),
        ("whatsapp", "g.whatsapp.net"),
    ]
}

fn connectivity_item(profile: &str, host: &str, port: u16) -> DiagnosticItem {
    let result = check_tcp(host, port);
    DiagnosticItem {
        id: format!("connectivity_{profile}_{host}").replace('.', "_"),
        title: format!("{profile}: {host}"),
        status: if result.ok {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
        problem: result.problem,
        action: Some(result.action),
    }
}

fn check_dns(host: &str) -> NetworkCheck {
    match (host, 443).to_socket_addrs() {
        Ok(addrs) => {
            if addrs.count() > 0 {
                NetworkCheck {
                    ok: true,
                    problem: None,
                    action: "DNS отвечает.".to_string(),
                }
            } else {
                NetworkCheck {
                    ok: false,
                    problem: Some(format!("DNS не вернул адрес для {host}.")),
                    action: "Проверьте DNS или включите Secure DNS в браузере.".to_string(),
                }
            }
        }
        Err(err) => NetworkCheck {
            ok: false,
            problem: Some(format!("DNS ошибка для {host}: {err}.")),
            action: "Проверьте интернет, DNS и активный VPN/proxy.".to_string(),
        },
    }
}

fn check_tcp(host: &str, port: u16) -> NetworkCheck {
    let addrs = match (host, port).to_socket_addrs() {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(err) => {
            return NetworkCheck {
                ok: false,
                problem: Some(format!("DNS ошибка для {host}: {err}.")),
                action: "Сначала исправьте DNS, затем повторите проверку доступности.".to_string(),
            };
        }
    };
    if addrs.is_empty() {
        return NetworkCheck {
            ok: false,
            problem: Some(format!("DNS не вернул адрес для {host}.")),
            action: "Проверьте DNS или включите Secure DNS в браузере.".to_string(),
        };
    }
    let timeout = Duration::from_millis(1200);
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return NetworkCheck {
                ok: true,
                problem: None,
                action: format!("DNS и TCP {port} отвечают."),
            };
        }
    }
    NetworkCheck {
        ok: false,
        problem: Some(format!("TCP {port} для {host} не отвечает.")),
        action: "Выключите режим, выберите другую стратегию на главной странице и включите снова. Для Telegram/WhatsApp начните с ALT, ALT3, Simple Fake или ALT5.".to_string(),
    }
}

fn tls_item(profile: &str, host: &str) -> DiagnosticItem {
    let result = check_tls(host);
    DiagnosticItem {
        id: format!("tls_{profile}_{host}").replace('.', "_"),
        title: format!("TLS {profile}: {host}"),
        status: if result.ok {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
        problem: result.problem,
        action: Some(result.action),
    }
}

#[cfg(windows)]
fn check_tls(host: &str) -> NetworkCheck {
    let host = host.trim();
    if !host
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return NetworkCheck {
            ok: false,
            problem: Some("Некорректный host для TLS проверки.".to_string()),
            action: "Проверьте endpoint в диагностике.".to_string(),
        };
    }
    let host_ps = powershell_single_quote(host);
    let script = format!(
        "$hostName = '{host_ps}'; \
         $tcp = New-Object System.Net.Sockets.TcpClient; \
         $iar = $tcp.BeginConnect($hostName, 443, $null, $null); \
         if (-not $iar.AsyncWaitHandle.WaitOne(5000, $false)) {{ $tcp.Close(); throw 'TCP timeout' }}; \
         $tcp.EndConnect($iar); \
         $ssl = New-Object System.Net.Security.SslStream($tcp.GetStream(), $false); \
         $ssl.AuthenticateAsClient($hostName); \
         \"tls_ok protocol=$($ssl.SslProtocol)\"; \
         $ssl.Dispose(); $tcp.Close();"
    );
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", &script]);
    command.creation_flags(CREATE_NO_WINDOW);
    match command.output() {
        Ok(output) if output.status.success() => NetworkCheck {
            ok: true,
            problem: None,
            action: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        Ok(output) => NetworkCheck {
            ok: false,
            problem: Some(format!(
                "TLS ошибка для {host}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            action:
                "Если DNS/TCP OK, но TLS fail, попробуйте другую стратегию и проверьте VPN/proxy."
                    .to_string(),
        },
        Err(err) => NetworkCheck {
            ok: false,
            problem: Some(format!("TLS проверка не запустилась для {host}: {err}.")),
            action: "Повторите диагностику или экспортируйте диагностический пакет.".to_string(),
        },
    }
}

#[cfg(not(windows))]
fn check_tls(_host: &str) -> NetworkCheck {
    NetworkCheck {
        ok: false,
        problem: Some("TLS проверка доступна только на Windows.".to_string()),
        action: "Запустите диагностику в установленном Windows приложении.".to_string(),
    }
}

#[cfg(windows)]
fn terminate_pid(pid: u32, _context: &Path) -> Result<()> {
    let handle = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return Err(last_win32_error(&format!(
            "OpenProcess({pid}) for terminate"
        )));
    }
    let ok = unsafe { TerminateProcess(handle, 0) };
    if ok == 0 {
        let err = last_win32_error(&format!("TerminateProcess({pid})"));
        unsafe {
            CloseHandle(handle);
        }
        return Err(err);
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if !pid_is_running(pid)? {
            unsafe {
                CloseHandle(handle);
            }
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            unsafe {
                CloseHandle(handle);
            }
            return Err(ZapretError::Operation(format!(
                "Failed to stop engine PID {pid}: process still running."
            )));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(windows)]
fn terminate_process_handle(handle: isize, pid: u32) -> Result<()> {
    let handle = handle as _;
    let ok = unsafe { TerminateProcess(handle, 0) };
    if ok == 0 {
        let err = last_win32_error(&format!("TerminateProcess(handle pid={pid})"));
        unsafe {
            CloseHandle(handle);
        }
        return Err(err);
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut exit_code = 0;
        let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
        if ok == 0 {
            let err = last_win32_error(&format!("GetExitCodeProcess(handle pid={pid})"));
            unsafe {
                CloseHandle(handle);
            }
            return Err(err);
        }
        if exit_code != STILL_ACTIVE as u32 {
            unsafe {
                CloseHandle(handle);
            }
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            unsafe {
                CloseHandle(handle);
            }
            return Err(ZapretError::Operation(format!(
                "Failed to stop engine PID {pid}: retained process handle still active."
            )));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(not(windows))]
fn terminate_pid(pid: u32, _context: &Path) -> Result<()> {
    Err(ZapretError::Operation(format!(
        "Failed to stop engine PID {pid}: Windows only."
    )))
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

#[cfg(test)]
mod tests {
    use super::{
        build_winws_launch, command_line_references_runtime_root, copy_dir_recursive,
        disable_state_after_cleanup, expand_strategy_vars, extract_winws_command,
        is_deprecated_strategy, powershell_single_quote, profile_launch_report,
        runtime_root_command_prefix, runtime_status_from_cleanup_state, split_windows_args,
        validate_strategy_profile_scope, windivert_driver_path_is_app_owned,
        windivert_report_has_running_driver, windivert_service_name_is_safe, ServiceClient,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use zapret_manager_core::{DiagnosticStatus, RuntimeStatus};

    #[test]
    fn extracts_direct_winws_command_from_strategy() {
        let source = r#"@echo off
call service.bat status_zapret
call service.bat check_updates
call service.bat load_game_filter
call service.bat load_user_lists
start "zapret: %~n0" /min "%BIN%winws.exe" --wf-tcp=%GameFilterTCP%
"#;

        let command = extract_winws_command(source).expect("command");
        let expanded = expand_strategy_vars(
            &command,
            Path::new("C:\\Runtime\\bin"),
            Path::new("C:\\Runtime\\lists"),
        );
        let args = split_windows_args(&expanded);

        assert_eq!(args[0], "C:\\Runtime\\bin\\winws.exe");
        assert_eq!(args[1], "--wf-tcp=65535");
        assert!(!expanded.contains("service.bat"));
        assert!(!expanded.contains("start \"zapret: %~n0\" /min"));
    }

    #[test]
    fn runtime_command_line_match_requires_path_boundary() {
        let runtime_root =
            PathBuf::from(r"C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime");
        assert_eq!(
            runtime_root_command_prefix(&runtime_root),
            r"c:\users\john smith\appdata\local\zapretmanager\engine-runtime\"
        );
        assert!(command_line_references_runtime_root(
            r#""C:\Program Files\Zapret Manager\engine\local\bin\winws.exe" --hostlist="C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime\run-1\lists\list-general.txt""#,
            &runtime_root,
        ));
        assert!(command_line_references_runtime_root(
            r#""C:/Program Files/Zapret Manager/engine/local/bin/winws.exe" --hostlist="C:/Users/John Smith/AppData/Local/ZapretManager/engine-runtime/run-1/lists/list-general.txt""#,
            &runtime_root,
        ));
        assert!(!command_line_references_runtime_root(
            r#""C:\Other\winws.exe" --hostlist="C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime-old\run-1\lists\list-general.txt""#,
            &runtime_root,
        ));
        assert!(!command_line_references_runtime_root(
            r#""C:\Other\winws.exe" --hostlist="C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime2\run-1\lists\list-general.txt""#,
            &runtime_root,
        ));
    }

    #[test]
    fn build_launch_log_contains_preflight_details() {
        let root = test_runtime_dir("launch-log");
        let bin = root.join("bin");
        let lists = root.join("lists");
        fs::create_dir_all(&bin).expect("bin");
        fs::create_dir_all(&lists).expect("lists");
        fs::write(bin.join("winws.exe"), b"stub").expect("winws");
        fs::write(bin.join("WinDivert.dll"), b"stub").expect("dll");
        fs::write(bin.join("WinDivert64.sys"), b"stub").expect("sys");
        fs::write(bin.join("cygwin1.dll"), b"stub").expect("cygwin");
        fs::write(lists.join("list-general.txt"), b"example.org").expect("hostlist");
        let bat = root.join("general.bat");
        fs::write(
            &bat,
            r#"start "zapret: %~n0" /min "%BIN%winws.exe" --hostlist="%LISTS%list-general.txt""#,
        )
        .expect("bat");

        let profiles = vec!["telegram".to_string(), "whatsapp".to_string()];
        let launch = build_winws_launch(&bat, &root, &root, "alt", &profiles).expect("launch");
        let log = fs::read_to_string(root.join("engine-launch.log")).expect("log");

        assert_eq!(launch.exe_path, bin.join("winws.exe"));
        assert!(launch
            .hostlists
            .iter()
            .any(|hostlist| hostlist.ends_with("list-general.txt")));
        assert!(log.contains("work_dir="));
        assert!(log.contains("selected_profiles=telegram,whatsapp"));
        assert!(log.contains("strategy_scope=general"));
        assert!(log.contains("profile_strategy_candidates=telegram="));
        assert!(log.contains("profile_filters_added=disabled_safe_mode"));
        assert!(log.contains("used_hostlists="));
        assert!(log.contains("used_ipsets="));
        assert!(log.contains("windivert_dll=true"));
        assert!(log.contains("windivert_sys=true"));
        assert!(log.contains("argv=1"));
        assert!(log.contains("command="));
        assert!(log.contains("preflight_ok=true"));
        assert!(log.contains("argv_list_begin"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_launch_preserves_runtime_paths_with_spaces() {
        let root = test_runtime_dir("John Smith launch path");
        let bin = root.join("bin");
        let lists = root.join("lists");
        fs::create_dir_all(&bin).expect("bin");
        fs::create_dir_all(&lists).expect("lists");
        fs::write(bin.join("winws.exe"), b"stub").expect("winws");
        fs::write(bin.join("WinDivert.dll"), b"stub").expect("dll");
        fs::write(bin.join("WinDivert64.sys"), b"stub").expect("sys");
        fs::write(bin.join("cygwin1.dll"), b"stub").expect("cygwin");
        fs::write(bin.join("tls_clienthello_www_google_com.bin"), b"stub").expect("tls");
        fs::write(lists.join("list-general.txt"), b"example.org").expect("hostlist");
        let bat = root.join("general.bat");
        fs::write(
            &bat,
            r#"start "zapret: %~n0" /min "%BIN%winws.exe" --hostlist="%LISTS%list-general.txt" --dpi-desync-fake-tls="%BIN%tls_clienthello_www_google_com.bin" --filter-tcp=443"#,
        )
        .expect("bat");

        let profiles = vec!["discord".to_string()];
        let launch = build_winws_launch(&bat, &root, &root, "alt", &profiles).expect("launch");
        let hostlist_arg = format!("--hostlist={}", lists.join("list-general.txt").display());
        let fake_tls_arg = format!(
            "--dpi-desync-fake-tls={}",
            bin.join("tls_clienthello_www_google_com.bin").display()
        );

        assert_eq!(launch.exe_path, bin.join("winws.exe"));
        assert!(root.to_string_lossy().contains(' '));
        assert!(launch.args.iter().all(|arg| !arg.contains('"')));
        assert!(launch.args.iter().any(|arg| arg == &hostlist_arg));
        assert!(launch.args.iter().any(|arg| arg == &fake_tls_arg));
        assert!(launch.args.iter().any(|arg| arg == "--filter-tcp=443"));

        let log = fs::read_to_string(root.join("engine-launch.log")).expect("log");
        assert!(log.contains("preflight_ok=true"));
        assert!(log.contains("argv_list_begin"));
        assert!(log.contains(&format!("arg[0]={}", bin.join("winws.exe").display())));
        assert!(log.contains(&format!("arg[1]={hostlist_arg}")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_launch_uses_bundled_binary_root_and_runtime_lists() {
        let engine_root = test_runtime_dir("bundled engine");
        let runtime_root = test_runtime_dir("runtime lists");
        let engine_bin = engine_root.join("bin");
        let runtime_lists = runtime_root.join("lists");
        fs::create_dir_all(&engine_bin).expect("engine bin");
        fs::create_dir_all(&runtime_lists).expect("runtime lists");
        fs::write(engine_bin.join("winws.exe"), b"stub").expect("winws");
        fs::write(engine_bin.join("WinDivert.dll"), b"stub").expect("dll");
        fs::write(engine_bin.join("WinDivert64.sys"), b"stub").expect("sys");
        fs::write(engine_bin.join("cygwin1.dll"), b"stub").expect("cygwin");
        fs::write(runtime_lists.join("list-general.txt"), b"example.org").expect("hostlist");
        let bat = engine_root.join("general.bat");
        fs::write(
            &bat,
            r#"start "zapret: %~n0" /min "%BIN%winws.exe" --hostlist="%LISTS%list-general.txt""#,
        )
        .expect("bat");

        let launch =
            build_winws_launch(&bat, &runtime_root, &engine_root, "general", &[]).expect("launch");

        assert_eq!(launch.exe_path, engine_bin.join("winws.exe"));
        assert_eq!(launch.work_dir, engine_bin);
        assert!(launch.args.iter().any(|arg| arg
            == &format!(
                "--hostlist={}",
                runtime_lists.join("list-general.txt").display()
            )));

        let log = fs::read_to_string(runtime_root.join("engine-launch.log")).expect("log");
        assert!(log.contains(&format!(
            "exe={}",
            engine_root.join("bin").join("winws.exe").display()
        )));
        assert!(log.contains(&format!(
            "referenced_file source=hostlist path={} exists=true",
            runtime_lists.join("list-general.txt").display()
        )));

        let _ = fs::remove_dir_all(engine_root);
        let _ = fs::remove_dir_all(runtime_root);
    }

    #[test]
    fn visible_strategies_parse_and_preflight_with_spaced_runtime_path() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("engine")
            .join("local");
        for (strategy, bat_name) in [
            ("general", "general.bat"),
            ("alt", "general (ALT).bat"),
            ("alt3", "general (ALT3).bat"),
            ("simple_fake", "general (SIMPLE FAKE).bat"),
            ("alt5", "general (ALT5).bat"),
            ("fake_tls_auto", "general (FAKE TLS AUTO).bat"),
            ("telegram_web", "web (TELEGRAM).bat"),
            ("whatsapp_web", "web (WHATSAPP).bat"),
        ] {
            let root = test_runtime_dir(&format!("John Smith {strategy}"));
            copy_dir_recursive(&source, &root).expect("runtime copy");
            let profiles = match strategy {
                "telegram_web" => vec!["telegram".to_string()],
                "whatsapp_web" => vec!["whatsapp".to_string()],
                _ => vec!["discord".to_string(), "youtube".to_string()],
            };
            let launch =
                build_winws_launch(&root.join(bat_name), &root, &root, strategy, &profiles)
                    .expect(strategy);
            let log = fs::read_to_string(root.join("engine-launch.log")).expect("log");

            assert_eq!(launch.exe_path, root.join("bin").join("winws.exe"));
            assert!(!launch.args.is_empty(), "{strategy} argv must not be empty");
            assert!(
                launch.args.iter().all(|arg| !arg.contains('"')),
                "{strategy} argv contains raw quote"
            );
            assert!(
                launch.args.iter().any(|arg| arg
                    .contains(&root.join("lists").display().to_string())
                    || arg.contains(&root.join("bin").display().to_string())),
                "{strategy} should reference runtime files as single args"
            );
            assert!(
                log.contains("preflight_ok=true"),
                "{strategy} preflight failed"
            );
            assert!(log.contains("argv_list_begin"));
            assert!(log.contains("app_version="));
            assert!(log.contains("build_id="));
            assert!(
                !log.contains("^!"),
                "{strategy} still contains cmd caret escape in direct argv log"
            );

            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn web_only_strategies_require_their_matching_single_profile() {
        assert!(validate_strategy_profile_scope("telegram_web", &["telegram".to_string()]).is_ok());
        assert!(validate_strategy_profile_scope("whatsapp_web", &["whatsapp".to_string()]).is_ok());
        assert!(
            validate_strategy_profile_scope("telegram_web", &["whatsapp".to_string()]).is_err()
        );
        assert!(validate_strategy_profile_scope(
            "whatsapp_web",
            &["whatsapp".to_string(), "telegram".to_string()]
        )
        .is_err());
        assert!(validate_strategy_profile_scope("telegram_web", &["common".to_string()]).is_err());
    }

    #[test]
    fn known_bad_strategies_are_not_reused_from_saved_settings() {
        assert!(is_deprecated_strategy("alt5"));
        assert!(is_deprecated_strategy("alt6"));
        assert!(!is_deprecated_strategy("telegram_web"));
        assert!(!is_deprecated_strategy("alt"));
    }

    #[test]
    fn web_only_launches_reference_only_the_matching_web_hostlist() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("engine")
            .join("local");
        for (strategy, bat_name, profile, hostlist) in [
            (
                "telegram_web",
                "web (TELEGRAM).bat",
                "telegram",
                "list-telegram-web.txt",
            ),
            (
                "whatsapp_web",
                "web (WHATSAPP).bat",
                "whatsapp",
                "list-whatsapp-web.txt",
            ),
        ] {
            let root = test_runtime_dir(&format!("web strategy {strategy}"));
            copy_dir_recursive(&source, &root).expect("runtime copy");
            let launch = build_winws_launch(
                &root.join(bat_name),
                &root,
                &root,
                strategy,
                &[profile.to_string()],
            )
            .expect(strategy);
            let expected_hostlist = root.join("lists").join(hostlist).display().to_string();
            let log = fs::read_to_string(root.join("engine-launch.log")).expect("log");

            assert_eq!(launch.hostlists, vec![expected_hostlist.clone()]);
            assert!(launch.ipsets.is_empty());
            assert!(log.contains(&format!("strategy_scope={profile}_web_only")));
            assert!(log.contains("profile_filters_added=web_hostlist_strategy"));
            assert!(log.contains(&format!("--hostlist={expected_hostlist}")));

            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn manual_snapshot_uses_data_root_not_current_dir() {
        let content_root = test_runtime_dir("snapshot content");
        let data_root = test_runtime_dir("snapshot data");
        fs::create_dir_all(&content_root).expect("content");
        fs::create_dir_all(&data_root).expect("data");
        let client = ServiceClient::new(content_root.clone(), data_root.clone());

        client.create_snapshot().expect("snapshot");

        let snapshots = data_root.join("snapshots");
        assert!(snapshots.is_dir());
        assert!(fs::read_dir(&snapshots).expect("snapshots").count() > 0);
        assert!(!content_root.join("snapshots").exists());

        let _ = fs::remove_dir_all(content_root);
        let _ = fs::remove_dir_all(data_root);
    }

    #[test]
    fn diagnostics_do_not_claim_unchecked_service_or_endpoint_access() {
        let content_root = test_runtime_dir("diagnostics content");
        let data_root = test_runtime_dir("diagnostics data");
        fs::create_dir_all(&content_root).expect("content");
        fs::create_dir_all(&data_root).expect("data");
        let mut client = ServiceClient::new(content_root.clone(), data_root.clone());
        client.enabled = true;

        let report = client.diagnostics();
        let by_id = |id: &str| {
            report
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| panic!("missing diagnostic item {id}"))
        };

        assert_eq!(by_id("service_installed").status, DiagnosticStatus::Skipped);
        assert_eq!(by_id("service_running").status, DiagnosticStatus::Skipped);
        assert_eq!(by_id("local_backend").status, DiagnosticStatus::Ok);
        assert_eq!(by_id("dns").status, DiagnosticStatus::Skipped);
        assert_eq!(by_id("internet").status, DiagnosticStatus::Skipped);
        assert_ne!(by_id("discord").status, DiagnosticStatus::Ok);
        assert_ne!(by_id("youtube").status, DiagnosticStatus::Ok);

        let _ = fs::remove_dir_all(content_root);
        let _ = fs::remove_dir_all(data_root);
    }

    #[test]
    fn powershell_quote_escapes_single_quotes() {
        assert_eq!(powershell_single_quote("C:\\A'B"), "C:\\A''B");
    }

    #[test]
    fn disable_state_only_clears_after_successful_cleanup() {
        let profiles = vec!["discord".to_string(), "youtube".to_string()];
        assert_eq!(
            disable_state_after_cleanup(true, &profiles, true),
            (false, Vec::new(), false)
        );
        assert_eq!(
            disable_state_after_cleanup(true, &profiles, false),
            (true, profiles.clone(), true)
        );
        assert_eq!(
            disable_state_after_cleanup(false, &Vec::new(), false),
            (true, Vec::new(), true)
        );
    }

    #[test]
    fn windivert_scope_matches_only_app_runtime_root() {
        let runtime_root =
            PathBuf::from(r"C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime");

        assert!(windivert_driver_path_is_app_owned(
            r#"\??\C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime\run-1784807924490\bin\WinDivert64.sys"#,
            &runtime_root
        ));
        assert!(windivert_driver_path_is_app_owned(
            r#""C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime\run-1\bin\WinDivert64.sys""#,
            &runtime_root
        ));
        assert!(!windivert_driver_path_is_app_owned(
            r"C:\Windows\System32\drivers\WinDivert64.sys",
            &runtime_root
        ));
        assert!(!windivert_driver_path_is_app_owned(
            r"C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime-old\run-1\bin\WinDivert64.sys",
            &runtime_root
        ));
        assert!(!windivert_driver_path_is_app_owned(
            r"C:\Users\John Smith\AppData\Local\ZapretManager\engine-runtime\..\Other\WinDivert64.sys",
            &runtime_root
        ));
        assert!(!windivert_driver_path_is_app_owned(
            r"C:\Users\John Smith\AppData\Local\OtherApp\engine-runtime\run-1\bin\WinDivert64.sys",
            &runtime_root
        ));
    }

    #[test]
    fn windivert_running_report_forces_cleanup_error_status() {
        assert_eq!(
            runtime_status_from_cleanup_state(false, false, "windivert_running=false"),
            RuntimeStatus::Disabled
        );
        assert_eq!(
            runtime_status_from_cleanup_state(
                false,
                false,
                r"name=WinDivert; state=Running; started=true; path=%LOCALAPPDATA%\ZapretManager\engine-runtime\run-1\bin\WinDivert64.sys"
            ),
            RuntimeStatus::Error
        );
        assert_eq!(
            runtime_status_from_cleanup_state(
                false,
                true,
                r"name=WinDivert; state=Running; started=true; path=%LOCALAPPDATA%\ZapretManager\engine-runtime\run-1\bin\WinDivert64.sys"
            ),
            RuntimeStatus::Running
        );
        assert_eq!(
            runtime_status_from_cleanup_state(
                false,
                false,
                r"app_owned_driver=true; name=WinDivert; state=Stopped; started=false; path=%LOCALAPPDATA%\ZapretManager\engine-runtime\run-1\bin\WinDivert64.sys"
            ),
            RuntimeStatus::Error
        );
        assert!(windivert_report_has_running_driver(
            r"name=WinDivert; state=Running; started=true; path=x"
        ));
        assert!(!windivert_report_has_running_driver(
            r"name=WinDivert; state=Stopped; started=false; path=x"
        ));
        assert_eq!(
            runtime_status_from_cleanup_state(false, false, "windivert_check_error=access denied"),
            RuntimeStatus::Error
        );
    }

    #[test]
    fn windivert_service_names_are_strictly_scoped() {
        assert!(windivert_service_name_is_safe("WinDivert"));
        assert!(windivert_service_name_is_safe("WinDivert14"));
        assert!(windivert_service_name_is_safe("WinDivert_1.4-test"));
        assert!(windivert_service_name_is_safe("WinDivert64"));
        assert!(windivert_service_name_is_safe("windivert"));
        assert!(windivert_service_name_is_safe("WINDIVERT64"));
        assert!(!windivert_service_name_is_safe(""));
        assert!(!windivert_service_name_is_safe("OtherWinDivert"));
        assert!(!windivert_service_name_is_safe("WinDivert & calc"));
        assert!(!windivert_service_name_is_safe("WinDivert\\evil"));
        assert!(!windivert_service_name_is_safe("WinDivert'"));
    }

    #[cfg(windows)]
    #[test]
    fn windivert_cleanup_root_validation_rejects_scope_escape() {
        let expected = super::data_root().join("engine-runtime");
        assert!(super::validate_windivert_cleanup_root(&expected).is_ok());
        assert!(super::validate_windivert_cleanup_root(&expected.join("run-1")).is_err());
        assert!(super::validate_windivert_cleanup_root(Path::new(r"C:\")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windivert_validation_accepts_stale_missing_app_owned_driver() {
        let expected = super::data_root().join("engine-runtime");
        let stale_driver = expected
            .join("run-stale")
            .join("bin")
            .join("WinDivert64.sys");
        assert_eq!(
            super::validate_app_owned_windivert_driver_path(
                &stale_driver.display().to_string(),
                &expected
            )
            .expect("stale driver path"),
            true
        );
    }

    #[test]
    fn profile_report_tracks_telegram_whatsapp_coverage() {
        let profiles = vec!["common".to_string()];
        let hostlists = vec!["C:\\Runtime\\lists\\list-general-user.txt".to_string()];
        let ipsets = vec!["C:\\Runtime\\lists\\ipset-all.txt".to_string()];
        let report = profile_launch_report(&profiles, "alt5", &hostlists, &ipsets);

        assert!(report.strategy_candidates.contains("telegram="));
        assert!(report.strategy_candidates.contains("whatsapp="));
        assert!(report.hostlist_coverage.contains("telegram:"));
        assert!(report.hostlist_coverage.contains("whatsapp:"));
        assert!(report
            .hostlist_coverage
            .contains("covered_by_list_general_user=true"));
        assert!(report
            .hostlist_coverage
            .contains("covered_by_profile_list=false"));
        assert!(report.hostlist_coverage.contains("covered_by_ipset=true"));
    }

    fn test_runtime_dir(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("zapret-manager-{name}-{suffix}"))
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
            DiagnosticStatus::Warning => Some(format!("{title}: требуется внимание.")),
            DiagnosticStatus::Error => Some(format!("{title}: ошибка.")),
            DiagnosticStatus::Skipped => Some(format!("{title}: проверка пропущена.")),
        },
        action: Some(action.to_string()),
    }
}

fn profile_order(id: &str) -> usize {
    match id {
        "discord" => 0,
        "youtube" => 1,
        "telegram" => 2,
        "whatsapp" => 3,
        "common" => 4,
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

#[cfg(windows)]
fn known_local_app_data_dir() -> Option<PathBuf> {
    unsafe {
        let mut path = std::ptr::null_mut();
        let result = SHGetKnownFolderPath(
            &FOLDERID_LocalAppData,
            KF_FLAG_DONT_VERIFY as u32,
            std::ptr::null_mut(),
            &mut path,
        );
        if result == S_OK && !path.is_null() {
            let value = wide_ptr_to_string(path);
            CoTaskMemFree(path.cast());
            Some(PathBuf::from(value))
        } else {
            CoTaskMemFree(path.cast());
            None
        }
    }
}

#[cfg(windows)]
fn data_root() -> PathBuf {
    known_local_app_data_dir()
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        })
        .join("ZapretManager")
}

#[cfg(not(windows))]
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
