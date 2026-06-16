use crate::errors::{Result, ZapretError};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedSource {
    pub name: String,
    pub base_url: String,
    pub pinned_manifest_sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedSources {
    pub sources: Vec<TrustedSource>,
}

impl TrustedSources {
    pub fn validate_url(&self, candidate: &str) -> Result<()> {
        let candidate_url =
            Url::parse(candidate).map_err(|err| ZapretError::Validation(err.to_string()))?;
        if candidate_url.scheme() != "https" && candidate_url.scheme() != "file" {
            return Err(ZapretError::UntrustedSource(candidate.to_string()));
        }
        let trusted = self.sources.iter().any(|source| {
            Url::parse(&source.base_url)
                .map(|base| candidate_url.as_str().starts_with(base.as_str()))
                .unwrap_or(false)
        });
        if !trusted {
            return Err(ZapretError::UntrustedSource(candidate.to_string()));
        }
        Ok(())
    }
}
