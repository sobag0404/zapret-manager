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
    Ok(UpdateStatus {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        strategy_version: "1.0.0".to_string(),
        last_checked: chrono::Utc::now().to_rfc3339(),
        channel: "stable".to_string(),
        message: "Mock manifest checked, updates not required.".to_string(),
    })
}

#[tauri::command]
pub fn apply_strategy_update() -> std::result::Result<UpdateStatus, String> {
    let mut status = check_strategy_updates()?;
    status.message = "Mock strategy update applied with backup.".to_string();
    Ok(status)
}

#[tauri::command]
pub fn rollback_strategy_update() -> std::result::Result<UpdateStatus, String> {
    let mut status = check_strategy_updates()?;
    status.message = "Mock strategy rollback completed.".to_string();
    Ok(status)
}
