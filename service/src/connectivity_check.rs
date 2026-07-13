use zapret_manager_core::{DiagnosticItem, DiagnosticStatus};

pub fn service_health_items() -> Vec<DiagnosticItem> {
    vec![
        skipped("internet", "Интернет проверка"),
        skipped("discord_https", "Discord доступность"),
        skipped("youtube_https", "YouTube доступность"),
        skipped("telegram_https", "Telegram доступность"),
    ]
}

fn skipped(id: &str, title: &str) -> DiagnosticItem {
    DiagnosticItem {
        id: id.to_string(),
        title: title.to_string(),
        status: DiagnosticStatus::Skipped,
        problem: Some(format!(
            "{title}: фактическая проверка доступности не запускалась."
        )),
        action: Some(
            "Запустите health-check DNS/TCP/HTTPS перед выводом о доступности сервиса.".to_string(),
        ),
    }
}
