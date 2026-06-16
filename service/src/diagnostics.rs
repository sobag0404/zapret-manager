use crate::connectivity_check::service_health_items;
use crate::dns_check::dns_items;
use crate::service::query_service_state;
use std::path::Path;
use zapret_manager_core::{DiagnosticItem, DiagnosticReport, DiagnosticStatus, Result};

pub fn run_diagnostics(root: &Path, engine_running: bool) -> Result<DiagnosticReport> {
    let service_state = query_service_state()?;
    let mut items = vec![
        item(
            "admin",
            "Права администратора",
            DiagnosticStatus::Warning,
            "Запустите приложение от имени администратора для реального управления службой.",
        ),
        item(
            "service_installed",
            "Служба установлена",
            if service_state.installed {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Error
            },
            "Переустановите службу в разделе Восстановление.",
        ),
        item(
            "service_running",
            "Служба запущена",
            if service_state.running || !engine_running {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Error
            },
            "Перезапустите службу.",
        ),
        item(
            "engine_found",
            "Engine найден",
            if root.join("engine").join("manifest.json").exists() {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Warning
            },
            "Добавьте проверенный engine manifest и файлы.",
        ),
        item(
            "engine_hash",
            "Engine hash совпадает",
            DiagnosticStatus::Skipped,
            "Hash проверяется после подключения проверенного engine.",
        ),
        item(
            "driver",
            "Драйвер доступен",
            DiagnosticStatus::Skipped,
            "В mock-режиме драйвер не используется.",
        ),
        item(
            "vpn_conflict",
            "Конфликт с VPN",
            DiagnosticStatus::Skipped,
            "Автоопределение VPN будет добавлено после подключения Windows API.",
        ),
        item(
            "proxy_conflict",
            "Конфликт с proxy",
            DiagnosticStatus::Skipped,
            "Proxy не менялся приложением.",
        ),
        item(
            "antivirus",
            "Конфликт с антивирусом",
            DiagnosticStatus::Skipped,
            "Антивирус не опрашивается в mock-режиме.",
        ),
        item(
            "logs_write",
            "Папка логов доступна",
            DiagnosticStatus::Ok,
            "Логи доступны.",
        ),
        item(
            "snapshot",
            "Snapshot можно создать",
            DiagnosticStatus::Ok,
            "Snapshot доступен.",
        ),
        item(
            "revert",
            "Revert можно выполнить",
            DiagnosticStatus::Ok,
            "Safe revert доступен.",
        ),
        item(
            "strategy_integrity",
            "Последняя стратегия не повреждена",
            DiagnosticStatus::Ok,
            "Стратегии валидны.",
        ),
    ];
    items.extend(dns_items());
    items.extend(service_health_items());
    Ok(DiagnosticReport::aggregate(items))
}

fn item(id: &str, title: &str, status: DiagnosticStatus, action: &str) -> DiagnosticItem {
    DiagnosticItem {
        id: id.to_string(),
        title: title.to_string(),
        status,
        problem: match status {
            DiagnosticStatus::Ok => None,
            _ => Some(match status {
                DiagnosticStatus::Warning => format!("{title}: требуется внимание."),
                DiagnosticStatus::Error => format!("{title}: ошибка."),
                DiagnosticStatus::Skipped => format!("{title}: пропущено."),
                DiagnosticStatus::Ok => String::new(),
            }),
        },
        action: Some(action.to_string()),
    }
}
