use crate::service_client::client;
use zapret_manager_core::Profile;

#[tauri::command]
pub fn list_profiles() -> std::result::Result<Vec<Profile>, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .list_profiles()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_profile_enabled(id: String, enabled: bool) -> std::result::Result<Vec<String>, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .set_profile_enabled(id, enabled)
        .map_err(|err| err.to_string())
}
