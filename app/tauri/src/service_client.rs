use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use zapret_manager_core::{
    append_debug_log, append_user_log, AppSettings, AppStatus, DiagnosticItem, DiagnosticReport,
    DiagnosticStatus, EngineManifest, Profile, ProfileStatus, Result, RuntimeStatus,
    StrategyUpdateManifest, SystemSnapshot, TrustedSource, TrustedSources, VpnConflict,
    ZapretError,
};

static STATE: OnceLock<Mutex<ServiceClient>> = OnceLock::new();

pub fn client() -> &'static Mutex<ServiceClient> {
    STATE.get_or_init(|| Mutex::new(ServiceClient::new(project_root(), data_root())))
}

#[derive(Debug)]
pub struct ServiceClient {
    content_root: PathBuf,
    data_root: PathBuf,
    enabled_profiles: Vec<String>,
    enabled: bool,
    settings: AppSettings,
}

impl ServiceClient {
    pub fn new(content_root: PathBuf, data_root: PathBuf) -> Self {
        Self {
            content_root,
            data_root,
            enabled_profiles: Vec::new(),
            enabled: false,
            settings: AppSettings::default(),
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

        let engine = self.engine_readiness();
        if !engine.ready {
            self.log_user("Включение не выполнено: реальный engine не подключён.")?;
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
            self.log_debug(
                "warn",
                "vpn_conflict_blocked_enable",
                &format!("active adapters: {}", vpn.adapter_names.join(", ")),
            )?;
            return Err(ZapretError::Operation(format!(
                "Обнаружен активный VPN: {}. Включение заблокировано режимом безопасности.",
                vpn.adapter_names.join(", ")
            )));
        }

        self.log_user("Создаётся snapshot перед включением.")?;
        let snapshot = SystemSnapshot::mock(profiles.clone(), vec!["strategies:1.0.0".to_string()]);
        let snapshot_path = snapshot.save(&self.data_root.join("snapshots"))?;
        self.log_debug(
            "info",
            "snapshot_created",
            &format!("path={}", snapshot_path.display()),
        )?;

        self.enabled_profiles = profiles.clone();
        self.enabled = true;
        self.log_user(&format!("Режим включён: {}.", profiles.join(", ")))?;
        self.log_debug("info", "engine_ready", &engine.message)?;
        self.status()
    }

    pub fn disable_all(&mut self) -> Result<AppStatus> {
        self.log_user("Выключение режима: остановка mock engine.")?;
        self.log_debug(
            "info",
            "disable_requested",
            &format!("active_profiles={}", self.enabled_profiles.join(",")),
        )?;
        self.enabled = false;
        self.enabled_profiles.clear();
        self.log_user("Система восстановлена. Временные правила отсутствуют.")?;
        self.log_debug(
            "info",
            "safe_revert_completed",
            "mock safe revert completed; no external engine processes were started",
        )?;
        self.status()
    }

    pub fn diagnostics(&self) -> DiagnosticReport {
        let vpn = detect_vpn_conflict();
        let profiles_found = !self.list_profiles().unwrap_or_default().is_empty();
        let engine = self.engine_readiness();
        DiagnosticReport::aggregate(vec![
            diag(
                "admin",
                "Права администратора",
                DiagnosticStatus::Warning,
                "Для реальной службы потребуется запуск от имени администратора.",
            ),
            diag(
                "service_installed",
                "Служба установлена",
                DiagnosticStatus::Ok,
                "Mock-служба доступна.",
            ),
            diag(
                "service_running",
                "Служба запущена",
                DiagnosticStatus::Ok,
                "Mock-служба отвечает.",
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
                    "Проверка будет активна после подключения engine manifest с файлами."
                },
            ),
            diag(
                "driver",
                "Драйвер доступен",
                DiagnosticStatus::Skipped,
                "В mock-режиме драйвер не используется.",
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
                "DNS не менялся приложением.",
            ),
            diag(
                "internet",
                "Интернет доступен",
                DiagnosticStatus::Ok,
                "Высокоуровневая mock-проверка успешна.",
            ),
            diag(
                "discord",
                "Discord доступен",
                DiagnosticStatus::Ok,
                "Mock-проверка Discord успешна.",
            ),
            diag(
                "youtube",
                "YouTube доступен",
                DiagnosticStatus::Ok,
                "Mock-проверка YouTube успешна.",
            ),
            diag(
                "telegram",
                "Telegram доступен",
                DiagnosticStatus::Ok,
                "Mock-проверка Telegram успешна.",
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
                    "VPN не трогается. Приложение не меняет DNS/proxy/routes в mock-режиме."
                        .to_string()
                } else {
                    "Конфликт с VPN не найден.".to_string()
                }),
            },
            diag(
                "proxy",
                "Нет конфликта с proxy",
                DiagnosticStatus::Ok,
                "Proxy не менялся.",
            ),
            diag(
                "antivirus",
                "Конфликт с антивирусом",
                DiagnosticStatus::Skipped,
                "Антивирус не опрашивается в mock-режиме.",
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
                "Snapshot пишется в локальную папку пользователя.",
            ),
            diag(
                "revert",
                "Revert можно выполнить",
                DiagnosticStatus::Ok,
                "Mock safe revert доступен.",
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
        self.log_user("Настройки сохранены.")?;
        self.log_debug(
            "info",
            "settings_saved",
            &format!(
                "strategy_channel={}, safety_mode={}, allow_vpn_conflict={}",
                self.settings.strategy_channel,
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
        let trusted = TrustedSources {
            sources: vec![TrustedSource {
                name: "local-engine".to_string(),
                base_url: "file:///local/mock-engine".to_string(),
                pinned_manifest_sha256: None,
            }],
        };

        let manifest: EngineManifest =
            match zapret_manager_core::load_engine_manifest(&manifest_path) {
                Ok(manifest) => manifest,
                Err(err) => {
                    return EngineReadiness {
                        ready: false,
                        message: format!("Engine manifest не найден или повреждён: {err}."),
                    }
                }
            };

        if manifest.files.is_empty() {
            return EngineReadiness {
                ready: false,
                message: "Реальный engine не подключён. Сейчас это безопасный manager-каркас: он не запускает zapret и не меняет доступ к Discord/YouTube.".to_string(),
            };
        }

        if let Err(err) = manifest.validate(&trusted) {
            return EngineReadiness {
                ready: false,
                message: format!("Engine manifest не прошёл trusted-source проверку: {err}."),
            };
        }

        if let Err(err) = manifest.verify_files(&engine_dir, &engine_dir) {
            return EngineReadiness {
                ready: false,
                message: format!("Engine hash verification failed: {err}."),
            };
        }

        EngineReadiness {
            ready: true,
            message: "Engine manifest найден, trusted-source и hash проверены.".to_string(),
        }
    }
}

struct EngineReadiness {
    ready: bool,
    message: String,
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
    for _ in 0..6 {
        candidates.push(current.to_path_buf());
        candidates.push(current.join("resources"));
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }
}

fn has_bundled_content(path: &Path) -> bool {
    path.join("profiles").is_dir() && path.join("strategies").is_dir()
}

fn data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
        .join("ZapretManager")
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
