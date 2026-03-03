//! Report data structures.

use crate::checks::types::CheckResult;
use crate::protocol::mcp_types::Implementation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A complete audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Stable report schema version.
    #[serde(default = "default_report_schema_version")]
    pub schema_version: String,
    /// AgentOx version that produced this report.
    pub agentox_version: String,
    /// Timestamp of the audit run.
    pub timestamp: String,
    /// Target that was audited (command or URL).
    pub target: String,
    /// Server info from the initialize handshake.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<Implementation>,
    /// Negotiated protocol version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    /// All check results.
    pub results: Vec<CheckResult>,
    /// Summary statistics.
    pub summary: AuditSummary,
}

/// Summary statistics for an audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub by_severity: BTreeMap<String, usize>,
    pub duration_ms: u64,
}

impl AuditReport {
    /// Build a report from check results.
    pub fn from_results(
        results: Vec<CheckResult>,
        target: String,
        server_info: Option<Implementation>,
        protocol_version: Option<String>,
        total_duration_ms: u64,
    ) -> Self {
        let total_checks = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total_checks - passed;

        let mut by_severity = BTreeMap::new();
        for r in &results {
            if !r.passed {
                *by_severity.entry(r.severity.to_string()).or_insert(0) += 1;
            }
        }

        Self {
            schema_version: default_report_schema_version(),
            agentox_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            target,
            server_info,
            protocol_version,
            results,
            summary: AuditSummary {
                total_checks,
                passed,
                failed,
                by_severity,
                duration_ms: total_duration_ms,
            },
        }
    }
}

fn default_report_schema_version() -> String {
    "1.0".to_string()
}
