//! JSON report renderer.

use crate::report::types::AuditReport;

/// Render an audit report as pretty-printed JSON.
pub fn render(report: &AuditReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
