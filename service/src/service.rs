use zapret_manager_core::{Result, ZapretError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceState {
    pub installed: bool,
    pub running: bool,
    pub mode: String,
}

pub fn query_service_state() -> Result<ServiceState> {
    Ok(ServiceState {
        installed: false,
        running: false,
        mode: "mock".to_string(),
    })
}

pub fn restart_service() -> Result<ServiceState> {
    let state = query_service_state()?;
    if !state.installed {
        return Err(ZapretError::Operation(
            "Zapret Manager service is not installed".to_string(),
        ));
    }
    Ok(ServiceState {
        running: true,
        ..state
    })
}
