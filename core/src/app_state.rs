use crate::profiles::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Disabled,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub status: RuntimeStatus,
    pub enabled_profiles: Vec<String>,
    pub profiles: Vec<Profile>,
    pub message: String,
}
