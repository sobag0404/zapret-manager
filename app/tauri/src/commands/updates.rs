use crate::service_client::client;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    pub app_version: String,
    pub strategy_version: String,
    pub last_checked: String,
    pub channel: String,
    pub message: String,
}

#[tauri::command]
pub fn check_strategy_updates() -> std::result::Result<UpdateStatus, String> {
    let manifest = client()
        .lock()
        .map_err(|err| err.to_string())?
        .load_strategy_update_manifest()
        .map_err(|err| err.to_string())?;
    let latest = manifest
        .entries
        .iter()
        .map(|entry| entry.version.as_str())
        .max()
        .unwrap_or("1.0.0")
        .to_string();
    Ok(UpdateStatus {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        strategy_version: latest,
        last_checked: chrono::Utc::now().to_rfc3339(),
        channel: "stable".to_string(),
        message: format!(
            "Manifest проверен: {} стратегий, hash/schema/trusted-source OK.",
            manifest.entries.len()
        ),
    })
}

#[tauri::command]
pub fn apply_strategy_update() -> std::result::Result<UpdateStatus, String> {
    let applied = client()
        .lock()
        .map_err(|err| err.to_string())?
        .apply_strategy_updates()
        .map_err(|err| err.to_string())?;
    let mut status = check_strategy_updates()?;
    status.message =
        format!("Стратегии применены: {applied}. Backup создан там, где была старая версия.");
    Ok(status)
}

#[tauri::command]
pub fn rollback_strategy_update() -> std::result::Result<UpdateStatus, String> {
    let restored = client()
        .lock()
        .map_err(|err| err.to_string())?
        .rollback_strategy_updates()
        .map_err(|err| err.to_string())?;
    let mut status = check_strategy_updates()?;
    status.message = if restored == 0 {
        "Rollback не нужен: backup пока не найден.".to_string()
    } else {
        format!("Rollback стратегий выполнен: {restored}.")
    };
    Ok(status)
}
