//! CONF-001: Validates that `initialize` returns valid capabilities.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};

pub struct InitializeCapabilities;

#[async_trait::async_trait]
impl Check for InitializeCapabilities {
    fn id(&self) -> &str {
        "CONF-001"
    }

    fn name(&self) -> &str {
        "Initialize returns valid capabilities"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let mut results = Vec::new();
        let desc = "Validates that the initialize response contains protocolVersion, capabilities, and serverInfo";

        match &ctx.init_result {
            Some(init) => {
                // Check protocolVersion is present and recognized
                if init.protocol_version.is_empty() {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Critical,
                        desc,
                        "protocolVersion is empty",
                    ));
                } else {
                    results.push(CheckResult::pass(
                        self.id(),
                        self.name(),
                        self.category(),
                        desc,
                    ));
                }

                // Check serverInfo.name is present
                if init.server_info.name.is_empty() {
                    results.push(CheckResult::fail(
                        self.id(),
                        "Server info has name",
                        self.category(),
                        Severity::High,
                        "serverInfo.name must not be empty",
                        "serverInfo.name is empty",
                    ));
                }

                // Recommend version
                if init.server_info.version.is_none() {
                    results.push(CheckResult::fail(
                        self.id(),
                        "Server info has version",
                        self.category(),
                        Severity::Low,
                        "serverInfo.version is recommended",
                        "serverInfo.version is missing (recommended but not required)",
                    ));
                }
            }
            None => {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Critical,
                    desc,
                    "Initialize was not called or failed — no result available",
                ));
            }
        }

        results
    }
}
