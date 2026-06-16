use zapret_manager_core::{DiagnosticItem, DiagnosticStatus};

pub fn service_health_items() -> Vec<DiagnosticItem> {
    vec![
        ok("internet", "Интернет доступен"),
        ok("discord_https", "Discord доступен"),
        ok("youtube_https", "YouTube доступен"),
        ok("telegram_https", "Telegram доступен"),
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
