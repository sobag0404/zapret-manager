use crate::engine_supervisor::verify_no_engine_processes;
use crate::service::restart_service;
use zapret_manager_core::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecoveryActionResult {
    pub action: String,
    pub ok: bool,
    pub message: String,
}

pub fn repair_service() -> Result<RecoveryActionResult> {
    let state = restart_service()?;
    Ok(RecoveryActionResult {
        action: "repair_service".to_string(),
        ok: state.running,
        message: "Mock service repair completed".to_string(),
    })
}

pub fn stop_engine() -> Result<RecoveryActionResult> {
    let status = verify_no_engine_processes()?;
    Ok(RecoveryActionResult {
        action: "stop_engine".to_string(),
        ok: !status.running,
        message: "Mock engine stopped".to_string(),
    })
}
