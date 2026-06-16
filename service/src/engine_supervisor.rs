use zapret_manager_core::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineProcessStatus {
    pub running: bool,
    pub remaining_processes: Vec<String>,
}

pub fn verify_no_engine_processes() -> Result<EngineProcessStatus> {
    Ok(EngineProcessStatus {
        running: false,
        remaining_processes: Vec::new(),
    })
}
