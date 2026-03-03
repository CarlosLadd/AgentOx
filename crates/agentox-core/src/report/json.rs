//! JSON report renderer.

use crate::report::types::AuditReport;
use sha2::{Digest, Sha256};

/// Render an audit report as pretty-printed JSON.
pub fn render(report: &AuditReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Compute a deterministic evidence signature over a report payload.
///
/// The digest is computed with the current report minus any existing
/// `evidence_signature` field to keep verification stable.
pub fn evidence_signature(
    report: &AuditReport,
) -> Result<crate::report::types::EvidenceSignature, serde_json::Error> {
    let mut cloned = report.clone();
    cloned.evidence_signature = None;
    let payload = serde_json::to_vec(&cloned)?;
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let digest_hex = format!("{:x}", hasher.finalize());
    Ok(crate::report::types::EvidenceSignature {
        algorithm: "sha256".to_string(),
        digest_hex,
        verifier: "agentox verify --report <FILE>".to_string(),
    })
}
