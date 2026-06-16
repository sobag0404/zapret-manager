use crate::errors::{Result, ZapretError};
use crate::profiles::ProfileStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyManifestEntry {
    pub id: String,
    pub profile_id: String,
    pub channel: ProfileStatus,
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub path: String,
    pub sha256: String,
    pub trusted_source: String,
    pub experimental_requires_consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyUpdateManifest {
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<StrategyManifestEntry>,
}

impl StrategyUpdateManifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "1" {
            return Err(ZapretError::Validation(
                "unsupported strategy manifest schema_version".to_string(),
            ));
        }
        if self.entries.is_empty() {
            return Err(ZapretError::Validation(
                "strategy manifest must include at least one entry".to_string(),
            ));
        }
        for entry in &self.entries {
            if entry.id.trim().is_empty() || entry.profile_id.trim().is_empty() {
                return Err(ZapretError::Validation(
                    "strategy manifest entry id/profile_id must not be empty".to_string(),
                ));
            }
            if entry.sha256.len() != 64 || !entry.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(ZapretError::Validation(format!(
                    "strategy manifest entry {} has invalid sha256",
                    entry.id
                )));
            }
            if entry.channel == ProfileStatus::Experimental && !entry.experimental_requires_consent
            {
                return Err(ZapretError::Validation(format!(
                    "experimental strategy {} must require consent",
                    entry.id
                )));
            }
        }
        Ok(())
    }
}

pub fn load_strategy_manifest(path: &Path) -> Result<StrategyUpdateManifest> {
    let text = fs::read_to_string(path).map_err(|source| crate::errors::io_error(path, source))?;
    let manifest: StrategyUpdateManifest =
        serde_json::from_str(&text).map_err(|source| crate::errors::json_error(path, source))?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_manifest() {
        let manifest = StrategyUpdateManifest {
            schema_version: "1".to_string(),
            generated_at: Utc::now(),
            entries: vec![StrategyManifestEntry {
                id: "discord-stable".to_string(),
                profile_id: "discord".to_string(),
                channel: ProfileStatus::Stable,
                version: "1.0.0".to_string(),
                updated_at: Utc::now(),
                path: "stable/discord.json".to_string(),
                sha256: "a".repeat(64),
                trusted_source: "local".to_string(),
                experimental_requires_consent: false,
            }],
        };
        assert!(manifest.validate().is_ok());
    }
}
