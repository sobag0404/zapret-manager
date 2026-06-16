use crate::diagnostics::run_diagnostics;
use crate::engine_runner::EngineRunner;
use crate::logging::{debug_log_path, user_log_path};
use crate::revert::safe_revert;
use crate::state_snapshot::create_snapshot;
use std::fs;
use std::path::PathBuf;
use zapret_manager_core::{
    append_debug_log, append_user_log, AppStatus, DiagnosticReport, Result, RuntimeStatus,
    SystemSnapshot,
};

pub struct ServiceApi {
    root: PathBuf,
    enabled_profiles: Vec<String>,
    engine: EngineRunner,
}

impl ServiceApi {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            enabled_profiles: Vec::new(),
            engine: EngineRunner::new(),
        }
    }

    pub fn status(&self) -> Result<AppStatus> {
        Ok(AppStatus {
            status: if self.engine.is_running() {
                RuntimeStatus::Running
            } else {
                RuntimeStatus::Disabled
            },
            enabled_profiles: self.enabled_profiles.clone(),
            profiles: crate::profile_runner::load_profiles(&self.root)?,
            message: if self.engine.is_running() {
                "Mock engine is running".to_string()
            } else {
                "Disabled".to_string()
            },
        })
    }

    pub fn enable(&mut self, profiles: Vec<String>) -> Result<AppStatus> {
        append_user_log(&user_log_path(&self.root), "Режим включается.")?;
        append_debug_log(
            &debug_log_path(&self.root),
            "info",
            "enable",
            "mock enable requested",
        )?;
        let snapshot = create_snapshot(&self.root, profiles.clone())?;
        snapshot.save(&self.root.join("snapshots"))?;
        self.engine.verify(&self.root)?;
        self.engine.start(&profiles)?;
        self.enabled_profiles = profiles;
        append_user_log(&user_log_path(&self.root), "Режим включён.")?;
        self.status()
    }

    pub fn disable_all(&mut self) -> Result<AppStatus> {
        append_user_log(&user_log_path(&self.root), "Режим выключается.")?;
        self.engine.stop()?;
        safe_revert(&self.root)?;
        self.enabled_profiles.clear();
        append_user_log(&user_log_path(&self.root), "Система восстановлена.")?;
        self.status()
    }

    pub fn run_diagnostics(&self) -> Result<DiagnosticReport> {
        run_diagnostics(&self.root, self.engine.is_running())
    }

    pub fn create_snapshot(&self) -> Result<SystemSnapshot> {
        let snapshot = create_snapshot(&self.root, self.enabled_profiles.clone())?;
        snapshot.save(&self.root.join("snapshots"))?;
        Ok(snapshot)
    }

    pub fn restore_snapshot(&mut self) -> Result<AppStatus> {
        safe_revert(&self.root)?;
        self.engine.stop()?;
        self.enabled_profiles.clear();
        self.status()
    }

    pub fn emergency_disable(&mut self) -> Result<AppStatus> {
        append_user_log(&user_log_path(&self.root), "Аварийное отключение запущено.")?;
        self.engine.stop()?;
        safe_revert(&self.root)?;
        fs::create_dir_all(self.root.join("logs"))
            .map_err(|source| zapret_manager_core::io_error(self.root.join("logs"), source))?;
        append_user_log(
            &user_log_path(&self.root),
            "Аварийное отключение завершено.",
        )?;
        self.enabled_profiles.clear();
        self.status()
    }
}
