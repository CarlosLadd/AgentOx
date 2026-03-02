//! CONF-008: Declared capabilities match actual method support.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};

pub struct CapabilityNegotiation;

#[async_trait::async_trait]
impl Check for CapabilityNegotiation {
    fn id(&self) -> &str {
        "CONF-008"
    }

    fn name(&self) -> &str {
        "Capability negotiation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server capabilities must match actually supported methods";
        let mut results = Vec::new();

        let caps = match &ctx.init_result {
            Some(init) => &init.capabilities,
            None => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    "No initialize result available",
                )];
            }
        };

        // If tools capability is declared, tools/list should work
        if caps.tools.is_some() {
            match ctx.session.list_tools().await {
                Ok(tools) => {
                    ctx.tools = Some(tools);
                }
                Err(e) => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Medium,
                        desc,
                        format!("Server declares tools capability but tools/list failed: {e}"),
                    ));
                }
            }
        }

        // If tools capability is NOT declared, tools/list should return error or empty
        if caps.tools.is_none() {
            match ctx.session.list_tools().await {
                Ok(tools) if !tools.is_empty() => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Medium,
                        desc,
                        format!(
                            "Server does not declare tools capability but tools/list returned {} tools",
                            tools.len()
                        ),
                    ));
                }
                _ => {} // Expected: error or empty
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
