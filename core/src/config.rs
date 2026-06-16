use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub autostart: bool,
    pub strategy_channel: String,
    pub logs_path: PathBuf,
    pub engine_path: PathBuf,
    pub safety_mode: bool,
    pub allow_vpn_conflict: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            strategy_channel: "stable".to_string(),
            logs_path: PathBuf::from("logs"),
            engine_path: PathBuf::from("engine/local"),
            safety_mode: true,
            allow_vpn_conflict: false,
        }
    }
}
