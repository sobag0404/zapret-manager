use std::path::{Path, PathBuf};

pub fn user_log_path(root: &Path) -> PathBuf {
    root.join("logs").join("user.log")
}

pub fn debug_log_path(root: &Path) -> PathBuf {
    root.join("logs").join("debug.jsonl")
}
