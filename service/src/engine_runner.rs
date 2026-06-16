use std::path::Path;
use zapret_manager_core::{Result, ZapretError};

#[derive(Debug, Default)]
pub struct EngineRunner {
    running: bool,
}

impl EngineRunner {
    pub fn new() -> Self {
        Self { running: false }
    }

    pub fn verify(&self, root: &Path) -> Result<()> {
        let manifest = root.join("engine").join("manifest.json");
        if !manifest.exists() {
            return Err(ZapretError::Operation(
                "engine manifest is missing; mock engine remains disabled".to_string(),
            ));
        }
        Ok(())
    }

    pub fn start(&mut self, profiles: &[String]) -> Result<()> {
        if profiles.is_empty() {
            return Err(ZapretError::Operation(
                "select at least one profile before enabling".to_string(),
            ));
        }
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}
