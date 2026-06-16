use crate::service_client::client;
use zapret_manager_core::AppStatus;

#[tauri::command]
pub fn toggle_enabled(profile_ids: Vec<String>) -> std::result::Result<AppStatus, String> {
    let mut guard = client().lock().map_err(|err| err.to_string())?;
    let status = guard.status().map_err(|err| err.to_string())?;
    if status.enabled_profiles.is_empty() && profile_ids.is_empty() {
        return Err("Выберите хотя бы один режим.".to_string());
    }
    match status.status {
        zapret_manager_core::RuntimeStatus::Running => guard.disable_all(),
        _ => guard.enable(if profile_ids.is_empty() {
            status.enabled_profiles
        } else {
            profile_ids
        }),
    }
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn enable_selected_profiles(
    profile_ids: Vec<String>,
) -> std::result::Result<AppStatus, String> {
    client()
        .lock()
        .map_err(|err| err.to_string())?
        .enable(profile_ids)
        .map_err(|err| err.to_string())
}
