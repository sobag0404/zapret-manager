use crate::errors::{Result, ZapretError};
use crate::hash_check::verify_sha256;
use crate::trusted_sources::TrustedSources;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineFile {
    pub relative_path: String,
    pub sha256: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineManifest {
    pub schema_version: String,
    pub engine_version: String,
    pub source_url: String,
    pub files: Vec<EngineFile>,
}

impl EngineManifest {
    pub fn validate(&self, trusted_sources: &TrustedSources) -> Result<()> {
        if self.schema_version != "1" {
            return Err(ZapretError::Validation(
                "unsupported engine manifest schema_version".to_string(),
            ));
        }
        trusted_sources.validate_url(&self.source_url)?;
        for file in &self.files {
            if file.relative_path.contains("..") || Path::new(&file.relative_path).is_absolute() {
                return Err(ZapretError::UnsafePath(PathBuf::from(&file.relative_path)));
            }
            if file.sha256.len() != 64 || !file.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(ZapretError::Validation(format!(
                    "engine file {} has invalid sha256",
                    file.relative_path
                )));
            }
        }
        Ok(())
    }

    pub fn verify_files(&self, engine_dir: &Path, allowed_dir: &Path) -> Result<()> {
        let allowed = allowed_dir
            .canonicalize()
            .map_err(|source| crate::errors::io_error(allowed_dir, source))?;
        for file in &self.files {
            let candidate = engine_dir.join(&file.relative_path);
            let canonical = candidate
                .canonicalize()
                .map_err(|source| crate::errors::io_error(&candidate, source))?;
            if !canonical.starts_with(&allowed) {
                return Err(ZapretError::UnsafePath(canonical));
            }
            verify_sha256(&candidate, &file.sha256)?;
        }
        Ok(())
    }
}

pub fn load_engine_manifest(path: &Path) -> Result<EngineManifest> {
    let text = fs::read_to_string(path).map_err(|source| crate::errors::io_error(path, source))?;
    serde_json::from_str(&text).map_err(|source| crate::errors::json_error(path, source))
}
