//! Policy-as-code evaluation and baseline regression utilities.

use crate::checks::types::{CheckCategory, Severity};
use crate::platform::types::AgentProtocol;
use crate::report::types::{AuditReport, PolicyDecision, PolicyDecisionStatus};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundle {
    #[serde(default = "default_policy_version")]
    pub version: String,
    #[serde(default)]
    pub default: PolicyGate,
    #[serde(default)]
    pub environments: BTreeMap<String, PolicyGate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyGate {
    #[serde(default)]
    pub fail_on: Vec<SeverityRule>,
    #[serde(default)]
    pub max_high: Option<usize>,
    #[serde(default)]
    pub max_medium: Option<usize>,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    #[serde(default)]
    pub check_id: Option<String>,
    #[serde(default)]
    pub category: Option<CheckCategory>,
    #[serde(default)]
    pub protocol: Option<AgentProtocol>,
    #[serde(default)]
    pub min_severity: Option<SeverityRule>,
    #[serde(default)]
    pub action: RuleAction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    #[default]
    Warn,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SeverityRule {
    Low,
    Medium,
    High,
    Critical,
}

impl SeverityRule {
    fn matches(self, severity: Severity) -> bool {
        match self {
            Self::Low => matches!(
                severity,
                Severity::Low | Severity::Medium | Severity::High | Severity::Critical
            ),
            Self::Medium => matches!(
                severity,
                Severity::Medium | Severity::High | Severity::Critical
            ),
            Self::High => matches!(severity, Severity::High | Severity::Critical),
            Self::Critical => matches!(severity, Severity::Critical),
        }
    }
}

impl From<SeverityRule> for Severity {
    fn from(value: SeverityRule) -> Self {
        match value {
            SeverityRule::Low => Severity::Low,
            SeverityRule::Medium => Severity::Medium,
            SeverityRule::High => Severity::High,
            SeverityRule::Critical => Severity::Critical,
        }
    }
}

fn default_policy_version() -> String {
    "1".to_string()
}

pub fn load_policy_file(path: impl AsRef<Path>) -> Result<PolicyBundle, anyhow::Error> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed reading policy file {}: {e}", path.display()))?;
    serde_yaml::from_str::<PolicyBundle>(&raw)
        .map_err(|e| anyhow::anyhow!("Failed parsing policy file {}: {e}", path.display()))
}

pub fn evaluate_report(
    report: &AuditReport,
    policy: &PolicyBundle,
    env_name: Option<&str>,
) -> PolicyDecision {
    let mut reasons = Vec::new();
    let mut status = PolicyDecisionStatus::Pass;

    let gate = env_name
        .and_then(|env| policy.environments.get(env))
        .unwrap_or(&policy.default);

    let mut high = 0usize;
    let mut medium = 0usize;

    for finding in report.results.iter().filter(|r| !r.passed) {
        if gate.fail_on.iter().any(|s| s.matches(finding.severity)) {
            reasons.push(format!(
                "{} exceeded fail_on threshold ({})",
                finding.check_id, finding.severity
            ));
            status = PolicyDecisionStatus::Fail;
        }

        match finding.severity {
            Severity::High => high += 1,
            Severity::Medium => medium += 1,
            _ => {}
        }

        for rule in &gate.rules {
            let check_match = rule
                .check_id
                .as_ref()
                .is_none_or(|id| id == &finding.check_id);
            let cat_match = rule.category.is_none_or(|c| c == finding.category);
            let protocol_match = rule.protocol.is_none_or(|p| report.protocol == Some(p));
            let sev_match = rule
                .min_severity
                .is_none_or(|s| s.matches(finding.severity));

            if check_match && cat_match && protocol_match && sev_match {
                match rule.action {
                    RuleAction::Fail => {
                        status = PolicyDecisionStatus::Fail;
                        reasons.push(format!(
                            "Rule fail matched for {} ({})",
                            finding.check_id, finding.severity
                        ));
                    }
                    RuleAction::Warn => {
                        if !matches!(status, PolicyDecisionStatus::Fail) {
                            status = PolicyDecisionStatus::Warn;
                        }
                        reasons.push(format!(
                            "Rule warn matched for {} ({})",
                            finding.check_id, finding.severity
                        ));
                    }
                }
            }
        }
    }

    if let Some(max_high) = gate.max_high {
        if high > max_high {
            status = PolicyDecisionStatus::Fail;
            reasons.push(format!(
                "HIGH findings {} exceeded max_high {}",
                high, max_high
            ));
        }
    }

    if let Some(max_medium) = gate.max_medium {
        if medium > max_medium {
            status = PolicyDecisionStatus::Fail;
            reasons.push(format!(
                "MEDIUM findings {} exceeded max_medium {}",
                medium, max_medium
            ));
        }
    }

    PolicyDecision { status, reasons }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDelta {
    #[serde(default)]
    pub new_failed_checks: Vec<String>,
    #[serde(default)]
    pub new_high_or_critical: Vec<String>,
}

pub fn compare_with_baseline(current: &AuditReport, baseline: &AuditReport) -> BaselineDelta {
    let baseline_failed: std::collections::HashSet<String> = baseline
        .results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.check_id.clone())
        .collect();

    let mut new_failed_checks = Vec::new();
    let mut new_high_or_critical = Vec::new();

    for finding in current.results.iter().filter(|r| !r.passed) {
        if !baseline_failed.contains(&finding.check_id) {
            new_failed_checks.push(finding.check_id.clone());
            if matches!(finding.severity, Severity::High | Severity::Critical) {
                new_high_or_critical.push(finding.check_id.clone());
            }
        }
    }

    new_failed_checks.sort();
    new_failed_checks.dedup();
    new_high_or_critical.sort();
    new_high_or_critical.dedup();

    BaselineDelta {
        new_failed_checks,
        new_high_or_critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::types::{CheckCategory, CheckResult, Severity};

    #[test]
    fn test_policy_fail_on_high() {
        let mut report = AuditReport::from_results(
            vec![CheckResult::fail(
                "SEC-999",
                "Synthetic High",
                CheckCategory::Security,
                Severity::High,
                "synthetic",
                "synthetic",
            )],
            "target".to_string(),
            None,
            None,
            1,
        );
        report.protocol = Some(AgentProtocol::Mcp);

        let policy = PolicyBundle {
            version: "1".to_string(),
            default: PolicyGate {
                fail_on: vec![SeverityRule::High],
                ..Default::default()
            },
            environments: BTreeMap::new(),
        };

        let decision = evaluate_report(&report, &policy, None);
        assert!(matches!(decision.status, PolicyDecisionStatus::Fail));
    }

    #[test]
    fn test_baseline_detects_new_high() {
        let current = AuditReport::from_results(
            vec![CheckResult::fail(
                "SEC-123",
                "Synthetic High",
                CheckCategory::Security,
                Severity::High,
                "synthetic",
                "synthetic",
            )],
            "target".to_string(),
            None,
            None,
            1,
        );
        let baseline = AuditReport::from_results(Vec::new(), "target".to_string(), None, None, 1);
        let delta = compare_with_baseline(&current, &baseline);
        assert_eq!(delta.new_failed_checks, vec!["SEC-123".to_string()]);
        assert_eq!(delta.new_high_or_critical, vec!["SEC-123".to_string()]);
    }
}
