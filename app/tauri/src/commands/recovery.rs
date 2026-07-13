use crate::{service_client::client, set_tray_status};
use zapret_manager_core::{AppStatus, SystemSnapshot};

#[tauri::command]
pub fn repair_driver() -> std::result::Result<String, String> {
    Ok("WinDivert проверяется только при запуске engine. Эта кнопка ничего не меняет в системе; если запуск блокируется, проверьте UAC и антивирус.".to_string())
}

#[tauri::command]
pub fn repair_service() -> std::result::Result<String, String> {
    Ok("Локальный backend доступен. Отдельная Windows-служба в v1.2 не переустанавливается этой кнопкой.".to_string())
}

#[tauri::command]
pub fn restart_engine() -> std::result::Result<String, String> {
    Ok("Автоматический restart не выполняется: сначала нажмите Выключить, затем Включить. Так сохраняется preflight и cleanup.".to_string())
}

#[tauri::command]
pub fn emergency_disable(app: tauri::AppHandle) -> std::result::Result<AppStatus, String> {
    let next = client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())?;
    set_tray_status(&app, false);
    Ok(next)
}

#[tauri::command]
pub fn create_snapshot() -> std::result::Result<SystemSnapshot, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .create_snapshot()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn restore_snapshot(app: tauri::AppHandle) -> std::result::Result<AppStatus, String> {
    let next = client()
        .lock()
        .map_err(|err| err.to_string())?
        .restore_snapshot()
        .map_err(|err| err.to_string())?;
    set_tray_status(&app, false);
    Ok(next)
}
