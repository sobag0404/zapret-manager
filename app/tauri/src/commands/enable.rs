use crate::{service_client::client, set_tray_status};
use zapret_manager_core::{AppStatus, RuntimeStatus};

#[tauri::command]
pub fn toggle_enabled(
    app: tauri::AppHandle,
    profile_ids: Vec<String>,
) -> std::result::Result<AppStatus, String> {
    let mut guard = client().lock().map_err(|err| err.to_string())?;
    let status = guard.status().map_err(|err| err.to_string())?;
    if status.enabled_profiles.is_empty() && profile_ids.is_empty() {
        return Err("Выберите хотя бы один режим.".to_string());
    }

    let next = match status.status {
        RuntimeStatus::Running => guard.disable_all(),
        _ => guard.enable(if profile_ids.is_empty() {
            status.enabled_profiles
        } else {
            profile_ids
        }),
    }
    .map_err(|err| err.to_string())?;

    set_tray_status(&app, next.status == RuntimeStatus::Running);
    Ok(next)
}

#[tauri::command]
pub fn enable_selected_profiles(
    app: tauri::AppHandle,
    profile_ids: Vec<String>,
) -> std::result::Result<AppStatus, String> {
    let next = client()
        .lock()
        .map_err(|err| err.to_string())?
        .enable(profile_ids)
        .map_err(|err| err.to_string())?;
    set_tray_status(&app, next.status == RuntimeStatus::Running);
    Ok(next)
}
