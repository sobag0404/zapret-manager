use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use zapret_manager_core::{
    append_debug_log, append_user_log, AppSettings, AppStatus, DiagnosticItem, DiagnosticReport,
    DiagnosticStatus, Profile, Result, RuntimeStatus, SystemSnapshot, VpnConflict, ZapretError,
};

static STATE: OnceLock<Mutex<ServiceClient>> = OnceLock::new();

pub fn client() -> &'static Mutex<ServiceClient> {
    STATE.get_or_init(|| Mutex::new(ServiceClient::new(project_root())))
}

#[derive(Debug)]
pub struct ServiceClient {
    root: PathBuf,
    enabled_profiles: Vec<String>,
    enabled: bool,
    settings: AppSettings,
}

impl ServiceClient {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
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
                "Работает в mock-режиме".to_string()
            } else {
                "Отключено".to_string()
            },
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<Profile>> {
        let dir = self.root.join("profiles");
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
            self.enabled_profiles.push(id);
        } else if !enabled {
            self.enabled_profiles.retain(|profile| profile != &id);
        }
        Ok(self.enabled_profiles.clone())
    }

    pub fn enable(&mut self, profiles: Vec<String>) -> Result<AppStatus> {
        if profiles.is_empty() {
            return Err(ZapretError::Operation(
                "Выберите хотя бы один режим.".to_string(),
            ));
        }
        let vpn = detect_vpn_conflict();
        if self.settings.safety_mode && !self.settings.allow_vpn_conflict && vpn.detected {
            append_debug_log(
                &self.root.join("logs").join("debug.jsonl"),
                "warn",
                "vpn_conflict_blocked_enable",
                &format!("active adapters: {}", vpn.adapter_names.join(", ")),
            )?;
            return Err(ZapretError::Operation(format!(
                "Обнаружен активный VPN: {}. Включение заблокировано режимом безопасности.",
                vpn.adapter_names.join(", ")
            )));
        }
        let snapshot = SystemSnapshot::mock(profiles.clone(), vec!["strategies:1.0.0".to_string()]);
        snapshot.save(&self.root.join("snapshots"))?;
        append_user_log(&self.root.join("logs").join("user.log"), "Режим включён.")?;
        append_debug_log(
            &self.root.join("logs").join("debug.jsonl"),
            "info",
            "enable",
            "mock service enabled without launching external engine",
        )?;
        self.enabled_profiles = profiles;
        self.enabled = true;
        self.status()
    }

    pub fn disable_all(&mut self) -> Result<AppStatus> {
        append_user_log(&self.root.join("logs").join("user.log"), "Режим выключен.")?;
        append_debug_log(
            &self.root.join("logs").join("debug.jsonl"),
            "info",
            "disable",
            "mock safe revert completed",
        )?;
        self.enabled = false;
        self.enabled_profiles.clear();
        self.status()
    }

    pub fn diagnostics(&self) -> DiagnosticReport {
        let vpn = detect_vpn_conflict();
        DiagnosticReport::aggregate(vec![
            diag(
                "admin",
                "Права администратора",
                DiagnosticStatus::Warning,
                "Запустите от имени администратора для реального управления службой.",
            ),
            diag(
                "service_installed",
                "Служба установлена",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "service_running",
                "Служба запущена",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "engine_found",
                "Engine найден",
                DiagnosticStatus::Warning,
                "Подключите проверенный engine manifest.",
            ),
            diag(
                "engine_hash",
                "Engine hash совпадает",
                DiagnosticStatus::Skipped,
                "Будет проверяться после подключения engine.",
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
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "strategy_valid",
                "Стратегии валидны",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "dns",
                "DNS работает",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "internet",
                "Интернет доступен",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "discord",
                "Discord доступен",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "youtube",
                "YouTube доступен",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "telegram",
                "Telegram доступен",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            DiagnosticItem {
                id: "vpn".to_string(),
                title: "Конфликт с VPN".to_string(),
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
                    "Не включайте режим одновременно с VPN или включите совместимость вручную в настройках.".to_string()
                } else {
                    "Действий не требуется.".to_string()
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
                "Нет конфликта с антивирусом",
                DiagnosticStatus::Skipped,
                "Антивирус не опрашивается.",
            ),
            diag(
                "logs",
                "Папка логов доступна",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "snapshot",
                "Snapshot можно создать",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "revert",
                "Revert можно выполнить",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
            diag(
                "strategy_integrity",
                "Последняя стратегия не повреждена",
                DiagnosticStatus::Ok,
                "Действий не требуется.",
            ),
        ])
    }

    pub fn read_user_logs(&self) -> Result<String> {
        let path = self.root.join("logs").join("user.log");
        if !path.exists() {
            return Ok("Лог пуст.".to_string());
        }
        fs::read_to_string(&path).map_err(|source| zapret_manager_core::io_error(path, source))
    }

    pub fn export_debug_logs(&self) -> Result<PathBuf> {
        let source = self.root.join("logs").join("debug.jsonl");
        let target = self.root.join("logs").join("debug-export.jsonl");
        if !source.exists() {
            fs::write(&source, "")
                .map_err(|source_err| zapret_manager_core::io_error(&source, source_err))?;
        }
        fs::copy(&source, &target)
            .map_err(|source_err| zapret_manager_core::io_error(&target, source_err))?;
        Ok(target)
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.clone()
    }

    pub fn save_settings(&mut self, settings: AppSettings) -> Result<AppSettings> {
        self.settings = settings;
        Ok(self.settings.clone())
    }
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

    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -ExpandProperty Name",
            ])
            .output();
        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let adapter_names = text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .filter(|line| {
                    let lower = line.to_ascii_lowercase();
                    [
                        "vpn",
                        "wireguard",
                        "openvpn",
                        "tap",
                        "tun",
                        "tailscale",
                        "zerotier",
                    ]
                    .iter()
                    .any(|marker| lower.contains(marker))
                })
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if !adapter_names.is_empty() {
                return VpnConflict {
                    detected: true,
                    adapter_names,
                    message: "VPN-like active adapter detected.".to_string(),
                };
            }
        }
    }

    VpnConflict::none()
}

fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
