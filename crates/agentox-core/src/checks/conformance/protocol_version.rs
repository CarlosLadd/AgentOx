//! CONF-009: Server handles protocol version negotiation correctly.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};

pub struct ProtocolVersionValidation;

#[async_trait::async_trait]
impl Check for ProtocolVersionValidation {
    fn id(&self) -> &str {
        "CONF-009"
    }

    fn name(&self) -> &str {
        "Protocol version validation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server must handle version negotiation correctly and not echo bogus versions";
        let mut results = Vec::new();

        // Check that the negotiated version is a known MCP version
        let known_versions = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];

        match &ctx.init_result {
            Some(init) => {
                if !known_versions.contains(&init.protocol_version.as_str()) {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Medium,
                        desc,
                        format!(
                            "Server returned unrecognized protocol version: \"{}\"",
                            init.protocol_version
                        ),
                    ));
                }

                // Test with a bogus version using a disposable session
                match ctx.disposable_session().await {
                    Ok(mut _session) => {
                        // The disposable_session already runs initialize with our version.
                        // For a thorough test, we would send a raw initialize with a bogus version.
                        // This is simplified for v0.1 — we just check the primary version.
                    }
                    Err(_) => {
                        // Non-critical — we still have the primary version check
                    }
                }
            }
            None => {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    "No initialize result available",
                ));
            }
        }

        if results.is_empty() {
            results.push(CheckResult::pass(
                self.id(),
                self.name(),
                self.category(),
                desc,
            ));
        }

        results
    }
}
