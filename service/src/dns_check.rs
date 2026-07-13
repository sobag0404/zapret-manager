use zapret_manager_core::{DiagnosticItem, DiagnosticStatus};

pub fn dns_items() -> Vec<DiagnosticItem> {
    vec![
        skipped("dns_current", "Текущий DNS"),
        skipped("dns_responds", "DNS отвечает"),
        skipped("dns_resolve_discord", "DNS Discord"),
        skipped("dns_resolve_youtube", "DNS YouTube"),
        skipped("dns_resolve_telegram", "DNS Telegram"),
    ]
}

fn skipped(id: &str, title: &str) -> DiagnosticItem {
    DiagnosticItem {
        id: id.to_string(),
        title: title.to_string(),
        status: DiagnosticStatus::Skipped,
        problem: Some(format!("{title}: фактическая DNS-проверка не запускалась.")),
        action: Some(
            "Запустите отдельную DNS-диагностику перед выводом о доступности.".to_string(),
        ),
    }
}
