use crate::app_state::RuntimeStatus;
use crate::errors::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub proxy_settings: Option<String>,
    pub dns_settings: Option<String>,
    pub active_profiles: Vec<String>,
    pub service_status: RuntimeStatus,
    pub engine_process_state: String,
    pub temporary_rules: Vec<String>,
    pub strategy_versions: Vec<String>,
}

impl SystemSnapshot {
    pub fn mock(active_profiles: Vec<String>, strategy_versions: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            proxy_settings: Some("unchanged".to_string()),
            dns_settings: Some("unchanged".to_string()),
            active_profiles,
            service_status: RuntimeStatus::Disabled,
            engine_process_state: "not_running".to_string(),
            temporary_rules: Vec::new(),
            strategy_versions,
        }
    }

    pub fn save(&self, dir: &Path) -> Result<std::path::PathBuf> {
        fs::create_dir_all(dir).map_err(|source| crate::errors::io_error(dir, source))?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)
            .map_err(|source| crate::errors::json_error(&path, source))?;
        fs::write(&path, json).map_err(|source| crate::errors::io_error(&path, source))?;
        Ok(path)
    }
}

pub fn load_snapshot(path: &Path) -> Result<SystemSnapshot> {
    let text = fs::read_to_string(path).map_err(|source| crate::errors::io_error(path, source))?;
    serde_json::from_str(&text).map_err(|source| crate::errors::json_error(path, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_snapshot_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let snapshot = SystemSnapshot::mock(vec!["discord".to_string()], vec!["1.0.0".to_string()]);
        let path = snapshot.save(dir.path()).expect("save");
        assert!(path.exists());
        let restored = load_snapshot(&path).expect("load");
        assert_eq!(restored.active_profiles, vec!["discord"]);
    }
}
