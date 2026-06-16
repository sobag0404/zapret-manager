use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserLogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub event: String,
    pub detail: String,
}

pub fn append_user_log(path: &Path, message: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| crate::errors::io_error(parent, source))?;
    }
    let entry = UserLogEntry {
        timestamp: Utc::now(),
        message: sanitize(message),
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| crate::errors::io_error(path, source))?;
    writeln!(
        file,
        "{} - {}",
        entry.timestamp.format("%H:%M"),
        entry.message
    )
    .map_err(|source| crate::errors::io_error(path, source))?;
    Ok(())
}

pub fn append_debug_log(path: &Path, level: &str, event: &str, detail: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| crate::errors::io_error(parent, source))?;
    }
    let entry = DebugLogEntry {
        timestamp: Utc::now(),
        level: level.to_string(),
        event: sanitize(event),
        detail: sanitize(detail),
    };
    let json =
        serde_json::to_string(&entry).map_err(|source| crate::errors::json_error(path, source))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| crate::errors::io_error(path, source))?;
    writeln!(file, "{json}").map_err(|source| crate::errors::io_error(path, source))?;
    Ok(())
}

pub fn sanitize(input: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    if lowered.contains("token=")
        || lowered.contains("cookie")
        || lowered.contains("password")
        || lowered.contains("authorization")
    {
        return "[redacted]".to_string();
    }
    input.to_string()
}
