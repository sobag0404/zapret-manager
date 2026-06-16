use std::path::Path;
use zapret_manager_core::{Result, SystemSnapshot};

pub fn create_snapshot(_root: &Path, active_profiles: Vec<String>) -> Result<SystemSnapshot> {
    Ok(SystemSnapshot::mock(
        active_profiles,
        vec!["strategies:1.0.0".to_string()],
    ))
}
