use crate::errors::{Result, ZapretError};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Stable,
    Experimental,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ProfileStatus,
    pub version: String,
    pub targets: Vec<String>,
    pub health_checks: Vec<String>,
    pub engine_profile_ref: String,
    pub fallback_profiles: Vec<String>,
    pub risk_level: RiskLevel,
    pub notes: String,
}

impl Profile {
    pub fn validate(&self) -> Result<()> {
        require_id("profile.id", &self.id)?;
        require_semver("profile.version", &self.version)?;
        require_non_empty("profile.name", &self.name)?;
        require_non_empty("profile.engine_profile_ref", &self.engine_profile_ref)?;
        if self.health_checks.is_empty() {
            return Err(ZapretError::Validation(
                "profile.health_checks must not be empty".to_string(),
            ));
        }
        for check in &self.health_checks {
            match check.as_str() {
                "dns" | "tcp" | "https" => {}
                other => {
                    return Err(ZapretError::Validation(format!(
                        "unsupported health check: {other}"
                    )))
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Strategy {
    pub id: String,
    pub profile_id: String,
    pub channel: ProfileStatus,
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub engine_profile_ref: String,
    pub health_checks: Vec<String>,
    pub fallback_strategy: String,
    pub rollback_supported: bool,
    pub notes: String,
}

impl Strategy {
    pub fn validate(&self) -> Result<()> {
        require_id("strategy.id", &self.id)?;
        require_id("strategy.profile_id", &self.profile_id)?;
        require_semver("strategy.version", &self.version)?;
        require_non_empty("strategy.engine_profile_ref", &self.engine_profile_ref)?;
        if self.health_checks.is_empty() {
            return Err(ZapretError::Validation(
                "strategy.health_checks must not be empty".to_string(),
            ));
        }
        if !self
            .notes
            .to_ascii_lowercase()
            .contains("no real low-level")
        {
            return Err(ZapretError::Validation(
                "initial strategies must explicitly avoid low-level parameters".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn load_profile(path: &Path) -> Result<Profile> {
    let text = fs::read_to_string(path).map_err(|source| crate::errors::io_error(path, source))?;
    let profile: Profile =
        serde_json::from_str(&text).map_err(|source| crate::errors::json_error(path, source))?;
    profile.validate()?;
    Ok(profile)
}

pub fn load_strategy(path: &Path) -> Result<Strategy> {
    let text = fs::read_to_string(path).map_err(|source| crate::errors::io_error(path, source))?;
    let strategy: Strategy =
        serde_json::from_str(&text).map_err(|source| crate::errors::json_error(path, source))?;
    strategy.validate()?;
    Ok(strategy)
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ZapretError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn require_id(field: &str, value: &str) -> Result<()> {
    require_non_empty(field, value)?;
    if !value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(ZapretError::Validation(format!(
            "{field} may contain lowercase ascii letters, digits, '-' and '_' only"
        )));
    }
    Ok(())
}

fn require_semver(field: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
        return Err(ZapretError::Validation(format!(
            "{field} must use x.y.z semver"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_profile() {
        let profile = Profile {
            id: "discord".to_string(),
            name: "Discord".to_string(),
            description: "Discord profile".to_string(),
            status: ProfileStatus::Stable,
            version: "1.0.0".to_string(),
            targets: vec!["desktop_app".to_string()],
            health_checks: vec!["dns".to_string(), "https".to_string()],
            engine_profile_ref: "discord-default".to_string(),
            fallback_profiles: vec!["discord-safe".to_string()],
            risk_level: RiskLevel::Medium,
            notes: "No low-level strategy values in initial scaffold".to_string(),
        };
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_profile_version() {
        let mut profile = Profile {
            id: "discord".to_string(),
            name: "Discord".to_string(),
            description: "Discord profile".to_string(),
            status: ProfileStatus::Stable,
            version: "1".to_string(),
            targets: vec!["desktop_app".to_string()],
            health_checks: vec!["dns".to_string()],
            engine_profile_ref: "discord-default".to_string(),
            fallback_profiles: vec![],
            risk_level: RiskLevel::Medium,
            notes: "No low-level strategy values in initial scaffold".to_string(),
        };
        assert!(profile.validate().is_err());
        profile.version = "1.0.0".to_string();
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn validates_strategy() {
        let strategy = Strategy {
            id: "discord-stable".to_string(),
            profile_id: "discord".to_string(),
            channel: ProfileStatus::Stable,
            version: "1.0.0".to_string(),
            updated_at: Utc::now(),
            engine_profile_ref: "discord-default".to_string(),
            health_checks: vec!["dns".to_string(), "tcp".to_string()],
            fallback_strategy: "discord-safe".to_string(),
            rollback_supported: true,
            notes: "No real low-level parameters in scaffold".to_string(),
        };
        assert!(strategy.validate().is_ok());
    }
}
