use crate::service_client::client;
use zapret_manager_core::AppStatus;

#[tauri::command]
pub fn get_app_status() -> std::result::Result<AppStatus, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .status()
        .map_err(|err| err.to_string())
}
