//! Report data structures.

use crate::checks::types::CheckResult;
use crate::platform::types::{AdapterMetadata, AgentProtocol, UnsupportedCheck};
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
    /// Protocol family used for this audit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<AgentProtocol>,
    /// Adapter metadata for this audit run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterMetadata>,
    /// Checks that were skipped due to protocol capability mismatch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsupported_checks: Vec<UnsupportedCheck>,
    /// Policy evaluation decision, if a policy bundle was provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_decision: Option<PolicyDecision>,
    /// Deterministic evidence signature metadata for this report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_signature: Option<EvidenceSignature>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecisionStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub status: PolicyDecisionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSignature {
    pub algorithm: String,
    pub digest_hex: String,
    pub verifier: String,
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
            protocol: None,
            adapter: None,
            unsupported_checks: Vec::new(),
            policy_decision: None,
            evidence_signature: None,
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

    pub fn with_protocol_metadata(
        mut self,
        protocol: AgentProtocol,
        adapter: AdapterMetadata,
        unsupported_checks: Vec<UnsupportedCheck>,
    ) -> Self {
        self.protocol = Some(protocol);
        self.adapter = Some(adapter);
        self.unsupported_checks = unsupported_checks;
        self
    }

    pub fn with_policy_decision(mut self, decision: PolicyDecision) -> Self {
        self.policy_decision = Some(decision);
        self
    }

    pub fn with_evidence_signature(mut self, signature: EvidenceSignature) -> Self {
        self.evidence_signature = Some(signature);
        self
    }
}

fn default_report_schema_version() -> String {
    "1.0".to_string()
}
