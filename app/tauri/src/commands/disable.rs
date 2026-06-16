use crate::service_client::client;
use zapret_manager_core::AppStatus;

#[tauri::command]
pub fn disable_all() -> std::result::Result<AppStatus, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .disable_all()
        .map_err(|err| err.to_string())
}
