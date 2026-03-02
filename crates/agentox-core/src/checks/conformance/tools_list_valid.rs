//! CONF-003: Validates that `tools/list` returns valid tool definitions.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use std::collections::HashSet;

pub struct ToolsListValid;

#[async_trait::async_trait]
impl Check for ToolsListValid {
    fn id(&self) -> &str {
        "CONF-003"
    }

    fn name(&self) -> &str {
        "tools/list returns valid tools"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Each tool must have a name and inputSchema; no duplicate names allowed";

        let tools = match &ctx.tools {
            Some(tools) => tools.clone(),
            None => match ctx.session.list_tools().await {
                Ok(tools) => {
                    ctx.tools = Some(tools.clone());
                    tools
                }
                Err(e) => {
                    return vec![CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("Failed to call tools/list: {e}"),
                    )];
                }
            },
        };

        let mut results = Vec::new();
        let mut seen_names = HashSet::new();

        for tool in &tools {
            // Check name is not empty
            if tool.name.is_empty() {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    "A tool has an empty name",
                ));
            }

            // Check for duplicates
            if !seen_names.insert(&tool.name) {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    format!("Duplicate tool name: \"{}\"", tool.name),
                ));
            }

            // Check inputSchema exists and is an object
            if !tool.input_schema.is_object() {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    format!("Tool \"{}\" inputSchema is not a JSON object", tool.name),
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
