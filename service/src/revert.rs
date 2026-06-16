use std::fs;
use std::path::Path;
use zapret_manager_core::{append_debug_log, Result, RevertPlan, SystemSnapshot};

pub fn safe_revert(root: &Path) -> Result<RevertPlan> {
    let snapshot =
        latest_snapshot(root)?.unwrap_or_else(|| SystemSnapshot::mock(Vec::new(), Vec::new()));
    let mut plan = RevertPlan::from_snapshot(&snapshot);
    for step in &mut plan.steps {
        step.completed = true;
    }
    append_debug_log(
        &root.join("logs").join("debug.jsonl"),
        "info",
        "safe_revert",
        "mock revert completed",
    )?;
    Ok(plan)
}

fn latest_snapshot(root: &Path) -> Result<Option<SystemSnapshot>> {
    let dir = root.join("snapshots");
    if !dir.exists() {
        return Ok(None);
    }
    let mut files = fs::read_dir(&dir)
        .map_err(|source| zapret_manager_core::io_error(&dir, source))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    files.sort();
    if let Some(path) = files.pop() {
        return Ok(Some(zapret_manager_core::load_snapshot(&path)?));
    }
    Ok(None)
}
