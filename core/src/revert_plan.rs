use crate::errors::Result;
use crate::snapshot::SystemSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevertStep {
    pub id: String,
    pub description: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevertPlan {
    pub snapshot_id: uuid::Uuid,
    pub steps: Vec<RevertStep>,
}

impl RevertPlan {
    pub fn from_snapshot(snapshot: &SystemSnapshot) -> Self {
        Self {
            snapshot_id: snapshot.id,
            steps: vec![
                step("stop_engine", "Stop engine process"),
                step("remove_rules", "Remove temporary rules"),
                step("restore_proxy", "Restore proxy settings"),
                step("restore_dns", "Restore DNS settings"),
                step("verify_processes", "Verify no engine processes remain"),
            ],
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.steps.is_empty() {
            return Err(crate::errors::ZapretError::Validation(
                "revert plan must include steps".to_string(),
            ));
        }
        Ok(())
    }
}

fn step(id: &str, description: &str) -> RevertStep {
    RevertStep {
        id: id.to_string(),
        description: description.to_string(),
        completed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_revert_plan() {
        let snapshot = SystemSnapshot::mock(Vec::new(), Vec::new());
        let plan = RevertPlan::from_snapshot(&snapshot);
        assert!(plan.validate().is_ok());
        assert!(plan.steps.iter().any(|step| step.id == "restore_dns"));
    }
}
