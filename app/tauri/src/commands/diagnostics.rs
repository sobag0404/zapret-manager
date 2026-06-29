use crate::service_client::client;
use zapret_manager_core::DiagnosticReport;

#[tauri::command]
pub fn run_diagnostics() -> std::result::Result<DiagnosticReport, String> {
    Ok(client()
        .lock()
        .map_err(|err| err.to_string())?
        .diagnostics())
}

#[tauri::command]
pub fn run_dns_check() -> std::result::Result<DiagnosticReport, String> {
    Ok(client()
        .lock()
        .map_err(|err| err.to_string())?
        .dns_diagnostics())
}

#[tauri::command]
pub fn run_service_connectivity_tests() -> std::result::Result<DiagnosticReport, String> {
    Ok(client()
        .lock()
        .map_err(|err| err.to_string())?
        .connectivity_diagnostics())
}
