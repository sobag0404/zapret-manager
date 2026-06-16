use crate::{service_client::client, set_tray_status};
use zapret_manager_core::AppStatus;

#[tauri::command]
pub fn disable_all(app: tauri::AppHandle) -> std::result::Result<AppStatus, String> {
    let next = client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())?;
    set_tray_status(&app, false);
    Ok(next)
}
