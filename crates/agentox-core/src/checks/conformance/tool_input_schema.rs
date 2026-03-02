//! CONF-004: Validates that each tool's inputSchema is valid JSON Schema with type "object".

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};

pub struct ToolInputSchemaValid;

#[async_trait::async_trait]
impl Check for ToolInputSchemaValid {
    fn id(&self) -> &str {
        "CONF-004"
    }

    fn name(&self) -> &str {
        "Tool inputSchema is valid JSON Schema"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Each tool's inputSchema must be valid JSON Schema with type \"object\"";

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

        for tool in &tools {
            let schema = &tool.input_schema;

            // Check type is "object"
            match schema.get("type").and_then(|t| t.as_str()) {
                Some("object") => {}
                Some(other) => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Medium,
                        desc,
                        format!(
                            "Tool \"{}\" inputSchema.type is \"{}\" (must be \"object\")",
                            tool.name, other
                        ),
                    ));
                    continue;
                }
                None => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Medium,
                        desc,
                        format!(
                            "Tool \"{}\" inputSchema is missing \"type\" field",
                            tool.name
                        ),
                    ));
                    continue;
                }
            }

            // Validate properties field if present
            if let Some(props) = schema.get("properties") {
                if !props.is_object() {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!(
                            "Tool \"{}\" inputSchema.properties is not an object",
                            tool.name
                        ),
                    ));
                }
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
