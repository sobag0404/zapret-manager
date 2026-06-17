use crate::{service_client::client, set_tray_status};
use zapret_manager_core::{AppStatus, SystemSnapshot};

#[tauri::command]
pub fn repair_driver() -> std::result::Result<String, String> {
    Ok("WinDivert проверяется при запуске engine. Если запуск блокируется, проверьте UAC и антивирус.".to_string())
}

#[tauri::command]
pub fn repair_service() -> std::result::Result<String, String> {
    Ok("Локальный backend доступен. Отдельная Windows-служба будет вынесена в следующий этап.".to_string())
}

#[tauri::command]
pub fn restart_engine() -> std::result::Result<String, String> {
    Ok("Остановите и снова включите режим. Engine запускается только после проверки manifest/hash.".to_string())
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
pub fn restore_snapshot(app: tauri::AppHandle) -> std::result::Result<AppStatus, String> {
    let next = client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())?;
    set_tray_status(&app, false);
    Ok(next)
}
