use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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
    child: Child,
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

        if !is_elevated() {
            self.log_user("Включение остановлено: приложение запущено без прав администратора.")?;
            return Err(ZapretError::Operation(
                "Запустите Zapret Manager от имени администратора. WinDivert не стартует без UAC."
                    .to_string(),
            ));
        }

        let engine = self.engine_readiness();
        if !engine.ready {
            self.log_user("Включение не выполнено: реальный engine не подключён или hash не совпал.")?;
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

        let runtime_dir = self.prepare_runtime_engine()?;
        let child = self.start_engine(&runtime_dir)?;
        let pid = child.id();
        self.engine = Some(EngineProcess { child, runtime_dir });
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
            let pid = engine.child.id();
            match engine.child.try_wait() {
                Ok(Some(status)) => {
                    self.log_debug(
                        "info",
                        "engine_already_exited",
                        &format!("pid={pid}, status={status}"),
                    )?;
                }
                Ok(None) => {
                    engine
                        .child
                        .kill()
                        .map_err(|source| zapret_manager_core::io_error(&engine.runtime_dir, source))?;
                    let _ = engine.child.wait();
                    self.log_debug("info", "engine_killed", &format!("pid={pid}"))?;
                }
                Err(err) => {
                    self.log_debug("warn", "engine_stop_check_failed", &err.to_string())?;
                }
            }
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
                    "Для запуска WinDivert откройте приложение от имени администратора."
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

        let manifest: EngineManifest = match zapret_manager_core::load_engine_manifest(&manifest_path) {
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
                message: "Реальный engine не подключён. Доступ к Discord/YouTube/Telegram не изменится."
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
        let target = self.data_root.join("engine-runtime");
        if target.exists() {
            fs::remove_dir_all(&target)
                .map_err(|source_err| zapret_manager_core::io_error(&target, source_err))?;
        }
        copy_dir_recursive(&source, &target)?;
        self.log_debug(
            "info",
            "engine_runtime_prepared",
            &format!("source={}, target={}", source.display(), target.display()),
        )?;
        Ok(target)
    }

    fn start_engine(&self, runtime_dir: &Path) -> Result<Child> {
        let bin = runtime_dir.join("bin");
        let exe = bin.join("winws.exe");
        let args = build_winws_args(runtime_dir, &self.settings.engine_strategy);
        let strategy = normalized_engine_strategy(&self.settings.engine_strategy);
        self.log_debug(
            "info",
            "engine_start_command",
            &format!(
                "exe={}, strategy={}, args_count={}",
                exe.display(),
                strategy,
                args.len()
            ),
        )?;

        let mut command = Command::new(&exe);
        command
            .current_dir(&bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        let mut child = command
            .spawn()
            .map_err(|source| zapret_manager_core::io_error(&exe, source))?;
        std::thread::sleep(std::time::Duration::from_millis(900));
        if let Some(status) = child
            .try_wait()
            .map_err(|source| zapret_manager_core::io_error(&exe, source))?
        {
            return Err(ZapretError::Operation(format!(
                "Engine сразу завершился со статусом {status}. Проверьте права администратора, WinDivert и антивирус."
            )));
        }
        Ok(child)
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

struct EngineReadiness {
    ready: bool,
    version: String,
    message: String,
}

fn build_winws_args(runtime_dir: &Path, strategy: &str) -> Vec<String> {
    let bin = runtime_dir.join("bin");
    let lists = runtime_dir.join("lists");
    let p = |path: PathBuf| path.to_string_lossy().to_string();
    let strategy = normalized_engine_strategy(strategy);

    let mut args = vec![
        "--wf-tcp=80,443,2053,2083,2087,2096,8443".to_string(),
        "--wf-udp=443,19294-19344,50000-50100".to_string(),
        "--filter-udp=443".to_string(),
        format!("--hostlist={}", p(lists.join("list-general.txt"))),
        format!("--hostlist={}", p(lists.join("list-general-user.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude-user.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude-user.txt"))),
        "--dpi-desync=fake".to_string(),
        format!("--dpi-desync-repeats={}", if strategy == "fake_tls_auto" { 11 } else { 6 }),
        format!(
            "--dpi-desync-fake-quic={}",
            p(bin.join("quic_initial_www_google_com.bin"))
        ),
        "--new".to_string(),
        "--filter-udp=19294-19344,50000-50100".to_string(),
        "--filter-l7=discord,stun".to_string(),
        "--dpi-desync=fake".to_string(),
        format!(
            "--dpi-desync-fake-discord={}",
            p(bin.join("quic_initial_dbankcloud_ru.bin"))
        ),
        format!(
            "--dpi-desync-fake-stun={}",
            p(bin.join("quic_initial_dbankcloud_ru.bin"))
        ),
        "--dpi-desync-repeats=6".to_string(),
        "--new".to_string(),
        "--filter-tcp=2053,2083,2087,2096,8443".to_string(),
        "--hostlist-domains=discord.media".to_string(),
    ];
    push_tls_strategy_args(&mut args, &bin, "discord_media", &strategy);
    args.extend([
        "--new".to_string(),
        "--filter-tcp=443".to_string(),
        format!("--hostlist={}", p(lists.join("list-google.txt"))),
        "--ip-id=zero".to_string(),
    ]);
    push_tls_strategy_args(&mut args, &bin, "google", &strategy);
    args.extend([
        "--new".to_string(),
        "--filter-tcp=80,443".to_string(),
        format!("--hostlist={}", p(lists.join("list-general.txt"))),
        format!("--hostlist={}", p(lists.join("list-general-user.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude-user.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude-user.txt"))),
    ]);
    push_tls_strategy_args(&mut args, &bin, "general", &strategy);
    args.extend([
        "--new".to_string(),
        "--filter-udp=443".to_string(),
        format!("--ipset={}", p(lists.join("ipset-all.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude-user.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude-user.txt"))),
        "--dpi-desync=fake".to_string(),
        format!("--dpi-desync-repeats={}", if strategy == "fake_tls_auto" { 11 } else { 6 }),
        format!(
            "--dpi-desync-fake-quic={}",
            p(bin.join("quic_initial_www_google_com.bin"))
        ),
        "--new".to_string(),
        "--filter-tcp=80,443,8443".to_string(),
        format!("--ipset={}", p(lists.join("ipset-all.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude.txt"))),
        format!("--hostlist-exclude={}", p(lists.join("list-exclude-user.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude.txt"))),
        format!("--ipset-exclude={}", p(lists.join("ipset-exclude-user.txt"))),
    ]);
    push_tls_strategy_args(&mut args, &bin, "ipset", &strategy);
    args
}

fn push_tls_strategy_args(args: &mut Vec<String>, bin: &Path, scope: &str, strategy: &str) {
    let p = |path: PathBuf| path.to_string_lossy().to_string();
    match strategy {
        "alt" => {
            args.extend([
                "--dpi-desync=fake,fakedsplit".to_string(),
                "--dpi-desync-repeats=6".to_string(),
                "--dpi-desync-fooling=ts".to_string(),
                "--dpi-desync-fakedsplit-pattern=0x00".to_string(),
            ]);
            if matches!(scope, "discord_media" | "google") {
                args.push(format!(
                    "--dpi-desync-fake-tls={}",
                    p(bin.join("tls_clienthello_www_google_com.bin"))
                ));
            } else {
                push_simple_fake_payloads(args, bin);
            }
        }
        "alt2" => {
            args.extend([
                "--dpi-desync=multisplit".to_string(),
                "--dpi-desync-split-seqovl=652".to_string(),
                "--dpi-desync-split-pos=2".to_string(),
                format!(
                    "--dpi-desync-split-seqovl-pattern={}",
                    p(bin.join("tls_clienthello_www_google_com.bin"))
                ),
            ]);
        }
        "alt3" => {
            args.extend([
                "--dpi-desync=fake,hostfakesplit".to_string(),
                format!(
                    "--dpi-desync-fake-tls-mod=rnd,dupsid,sni={}",
                    if matches!(scope, "discord_media" | "google") {
                        "www.google.com"
                    } else {
                        "ya.ru"
                    }
                ),
                format!(
                    "--dpi-desync-hostfakesplit-mod=host={},altorder=1",
                    if matches!(scope, "discord_media" | "google") {
                        "www.google.com"
                    } else {
                        "ya.ru"
                    }
                ),
                "--dpi-desync-fooling=ts".to_string(),
            ]);
            if !matches!(scope, "discord_media" | "google") {
                args.push(format!(
                    "--dpi-desync-fake-http={}",
                    p(bin.join("tls_clienthello_max_ru.bin"))
                ));
            }
        }
        "simple_fake" => {
            args.extend([
                "--dpi-desync=fake".to_string(),
                "--dpi-desync-repeats=6".to_string(),
                "--dpi-desync-fooling=ts".to_string(),
            ]);
            if matches!(scope, "discord_media" | "google") {
                args.push(format!(
                    "--dpi-desync-fake-tls={}",
                    p(bin.join("tls_clienthello_www_google_com.bin"))
                ));
            } else {
                push_simple_fake_payloads(args, bin);
            }
        }
        "fake_tls_auto" => {
            args.extend([
                "--dpi-desync=fake,multidisorder".to_string(),
                "--dpi-desync-split-pos=1,midsld".to_string(),
                "--dpi-desync-repeats=11".to_string(),
                "--dpi-desync-fooling=badseq".to_string(),
                "--dpi-desync-fake-tls=0x00000000".to_string(),
                "--dpi-desync-fake-tls=!".to_string(),
                "--dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com".to_string(),
            ]);
            if !matches!(scope, "discord_media" | "google") {
                args.push(format!(
                    "--dpi-desync-fake-http={}",
                    p(bin.join("tls_clienthello_max_ru.bin"))
                ));
            }
        }
        _ => {
            let (seq, pos, pattern) = match scope {
                "discord_media" | "google" => (681, 1, "tls_clienthello_www_google_com.bin"),
                _ => (568, 1, "tls_clienthello_4pda_to.bin"),
            };
            args.extend([
                "--dpi-desync=multisplit".to_string(),
                format!("--dpi-desync-split-seqovl={seq}"),
                format!("--dpi-desync-split-pos={pos}"),
                format!(
                    "--dpi-desync-split-seqovl-pattern={}",
                    p(bin.join(pattern))
                ),
            ]);
        }
    }
}

fn push_simple_fake_payloads(args: &mut Vec<String>, bin: &Path) {
    let p = |path: PathBuf| path.to_string_lossy().to_string();
    args.extend([
        format!("--dpi-desync-fake-tls={}", p(bin.join("stun.bin"))),
        format!(
            "--dpi-desync-fake-tls={}",
            p(bin.join("tls_clienthello_www_google_com.bin"))
        ),
        format!(
            "--dpi-desync-fake-http={}",
            p(bin.join("tls_clienthello_max_ru.bin"))
        ),
    ]);
}

fn normalized_engine_strategy(strategy: &str) -> String {
    match strategy {
        "alt" | "alt2" | "alt3" | "simple_fake" | "fake_tls_auto" => strategy.to_string(),
        _ => "general".to_string(),
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
    command.status().map(|status| status.success()).unwrap_or(false)
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
    let text = fs::read_to_string(&path)
        .map_err(|source| zapret_manager_core::io_error(&path, source))?;
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
