//! Types shared across all audit checks.

use serde::{Deserialize, Serialize};

/// Severity levels for check results, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Pass,
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// The category of a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckCategory {
    Conformance,
    Security,
    Behavioral,
}

/// The result of a single check execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Unique identifier for this check (e.g., "CONF-001").
    pub check_id: String,
    /// Human-readable check name.
    pub name: String,
    /// Category (conformance, security, behavioral).
    pub category: CheckCategory,
    /// Severity of the finding.
    pub severity: Severity,
    /// Did the check pass?
    pub passed: bool,
    /// Human-readable description of what was checked.
    pub description: String,
    /// Detailed message about the finding.
    pub message: String,
    /// Optional evidence (raw JSON, response data, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
    /// Duration of the check in milliseconds.
    pub duration_ms: u64,
}

impl CheckResult {
    /// Create a passing result.
    pub fn pass(
        check_id: impl Into<String>,
        name: impl Into<String>,
        category: CheckCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            name: name.into(),
            category,
            severity: Severity::Pass,
            passed: true,
            description: description.into(),
            message: "Check passed".to_string(),
            evidence: None,
            duration_ms: 0,
        }
    }

    /// Create a failing result.
    pub fn fail(
        check_id: impl Into<String>,
        name: impl Into<String>,
        category: CheckCategory,
        severity: Severity,
        description: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            name: name.into(),
            category,
            severity,
            passed: false,
            description: description.into(),
            message: message.into(),
            evidence: None,
            duration_ms: 0,
        }
    }

    /// Attach evidence to this result.
    pub fn with_evidence(mut self, evidence: serde_json::Value) -> Self {
        self.evidence = Some(evidence);
        self
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Pass => write!(f, "PASS"),
            Severity::Info => write!(f, "INFO"),
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}
