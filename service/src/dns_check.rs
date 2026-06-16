use zapret_manager_core::{DiagnosticItem, DiagnosticStatus};

pub fn dns_items() -> Vec<DiagnosticItem> {
    vec![
        ok("dns_current", "Текущий DNS определён"),
        ok("dns_responds", "DNS отвечает"),
        ok("dns_resolve_discord", "DNS Discord"),
        ok("dns_resolve_youtube", "DNS YouTube"),
        ok("dns_resolve_telegram", "DNS Telegram"),
    ]
}

fn ok(id: &str, title: &str) -> DiagnosticItem {
    DiagnosticItem {
        id: id.to_string(),
        title: title.to_string(),
        status: DiagnosticStatus::Ok,
        problem: None,
        action: Some("Действий не требуется.".to_string()),
    }
}
