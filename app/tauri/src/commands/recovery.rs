use crate::service_client::client;
use zapret_manager_core::{AppStatus, SystemSnapshot};

#[tauri::command]
pub fn repair_driver() -> std::result::Result<String, String> {
    Ok("Mock: драйвер не используется, проверка пропущена.".to_string())
}

#[tauri::command]
pub fn repair_service() -> std::result::Result<String, String> {
    Ok("Mock: служба проверена.".to_string())
}

#[tauri::command]
pub fn restart_engine() -> std::result::Result<String, String> {
    Ok("Mock: engine перезапущен без запуска внешних бинарников.".to_string())
}

#[tauri::command]
pub fn emergency_disable() -> std::result::Result<AppStatus, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn create_snapshot() -> std::result::Result<SystemSnapshot, String> {
    let status = client()
        .lock()
        .map_err(|err| err.to_string())?
        .status()
        .map_err(|err| err.to_string())?;
    let snapshot = SystemSnapshot::mock(
        status.enabled_profiles,
        vec!["strategies:1.0.0".to_string()],
    );
    snapshot
        .save(
            &std::env::current_dir()
                .map_err(|err| err.to_string())?
                .join("snapshots"),
        )
        .map_err(|err| err.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
pub fn restore_snapshot() -> std::result::Result<AppStatus, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())
}
