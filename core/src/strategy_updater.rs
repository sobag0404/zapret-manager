use crate::errors::{Result, ZapretError};
use crate::hash_check::verify_sha256;
use crate::profiles::load_strategy;
use crate::trusted_sources::TrustedSources;
use crate::update_manifest::{StrategyManifestEntry, StrategyUpdateManifest};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StrategyUpdateResult {
    pub applied: Vec<String>,
    pub backed_up: Vec<PathBuf>,
}

pub fn check_updates(manifest: &StrategyUpdateManifest) -> Result<Vec<StrategyManifestEntry>> {
    manifest.validate()?;
    Ok(manifest.entries.clone())
}

pub fn apply_strategy_update(
    manifest: &StrategyUpdateManifest,
    source_dir: &Path,
    strategies_dir: &Path,
    backup_dir: &Path,
    trusted_sources: &TrustedSources,
) -> Result<StrategyUpdateResult> {
    manifest.validate()?;
    fs::create_dir_all(backup_dir).map_err(|source| crate::errors::io_error(backup_dir, source))?;
    let mut result = StrategyUpdateResult {
        applied: Vec::new(),
        backed_up: Vec::new(),
    };
    for entry in &manifest.entries {
        trusted_sources.validate_url(&entry.trusted_source)?;
        let source = source_dir.join(&entry.path);
        verify_sha256(&source, &entry.sha256)?;
        let strategy = load_strategy(&source)?;
        if strategy.id != entry.id {
            return Err(ZapretError::Validation(format!(
                "manifest id {} does not match strategy {}",
                entry.id, strategy.id
            )));
        }
        let target = strategies_dir.join(&entry.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| crate::errors::io_error(parent, source))?;
        }
        if target.exists() {
            let backup = backup_dir.join(entry.path.replace(['/', '\\'], "_"));
            fs::copy(&target, &backup)
                .map_err(|source| crate::errors::io_error(&backup, source))?;
            result.backed_up.push(backup);
        }
        fs::copy(&source, &target).map_err(|source| crate::errors::io_error(&target, source))?;
        result.applied.push(entry.id.clone());
    }
    Ok(result)
}

pub fn rollback_strategy(backup_file: &Path, target_file: &Path) -> Result<()> {
    if !backup_file.exists() {
        return Err(ZapretError::Operation(format!(
            "backup does not exist: {}",
            backup_file.display()
        )));
    }
    fs::copy(backup_file, target_file)
        .map_err(|source| crate::errors::io_error(target_file, source))?;
    load_strategy(target_file)?;
    Ok(())
}
