use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Ok,
    Warning,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticItem {
    pub id: String,
    pub title: String,
    pub status: DiagnosticStatus,
    pub problem: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticReport {
    pub overall: DiagnosticStatus,
    pub items: Vec<DiagnosticItem>,
}

impl DiagnosticReport {
    pub fn aggregate(items: Vec<DiagnosticItem>) -> Self {
        let overall = if items
            .iter()
            .any(|item| item.status == DiagnosticStatus::Error)
        {
            DiagnosticStatus::Error
        } else if items
            .iter()
            .any(|item| item.status == DiagnosticStatus::Warning)
        {
            DiagnosticStatus::Warning
        } else if items
            .iter()
            .all(|item| item.status == DiagnosticStatus::Skipped)
        {
            DiagnosticStatus::Skipped
        } else {
            DiagnosticStatus::Ok
        };
        Self { overall, items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_results() {
        let report = DiagnosticReport::aggregate(vec![
            DiagnosticItem {
                id: "dns".to_string(),
                title: "DNS".to_string(),
                status: DiagnosticStatus::Ok,
                problem: None,
                action: None,
            },
            DiagnosticItem {
                id: "engine".to_string(),
                title: "Engine".to_string(),
                status: DiagnosticStatus::Warning,
                problem: Some("Engine mock".to_string()),
                action: Some("Install verified engine".to_string()),
            },
        ]);
        assert_eq!(report.overall, DiagnosticStatus::Warning);
    }
}
