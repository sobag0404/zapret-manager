use crate::service_client::client;

#[tauri::command]
pub fn read_user_logs() -> std::result::Result<String, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .read_user_logs()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn export_debug_logs() -> std::result::Result<String, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .export_debug_logs()
        .map(|path| path.display().to_string())
        .map_err(|err| err.to_string())
}
