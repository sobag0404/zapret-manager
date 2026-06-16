#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct VpnConflict {
    pub detected: bool,
    pub adapter_names: Vec<String>,
    pub message: String,
}

impl VpnConflict {
    pub fn none() -> Self {
        Self {
            detected: false,
            adapter_names: Vec::new(),
            message: "VPN conflict was not detected.".to_string(),
        }
    }
}
