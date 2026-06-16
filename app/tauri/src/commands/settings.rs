use crate::service_client::client;
use zapret_manager_core::AppSettings;

#[tauri::command]
pub fn get_settings() -> std::result::Result<AppSettings, String> {
    Ok(client().lock().map_err(|err| err.to_string())?.settings())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> std::result::Result<AppSettings, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .save_settings(settings)
        .map_err(|err| err.to_string())
}
