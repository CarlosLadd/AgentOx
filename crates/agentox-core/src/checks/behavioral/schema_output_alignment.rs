//! BHV-002: Validate tools/call output against declared outputSchema.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::mcp_types::Tool;

pub struct SchemaOutputAlignment;

fn default_value_for_schema(schema: &serde_json::Value) -> Option<serde_json::Value> {
    match schema.get("type").and_then(|v| v.as_str()) {
        Some("string") => Some(serde_json::Value::String("x".to_string())),
        Some("number") | Some("integer") => Some(serde_json::json!(1)),
        Some("boolean") => Some(serde_json::json!(false)),
        Some("object") => Some(serde_json::json!({})),
        Some("array") => Some(serde_json::json!([])),
        _ => None,
    }
}

fn build_minimal_args(input_schema: &serde_json::Value) -> serde_json::Value {
    let mut args = serde_json::Map::new();
    let required: Vec<String> = input_schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    let properties = input_schema
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    for req in required {
        if let Some(schema) = properties.get(&req) {
            if let Some(default) = default_value_for_schema(schema) {
                args.insert(req, default);
            }
        }
    }

    serde_json::Value::Object(args)
}

#[async_trait::async_trait]
impl Check for SchemaOutputAlignment {
    fn id(&self) -> &str {
        "BHV-002"
    }

    fn name(&self) -> &str {
        "Schema-output alignment"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Behavioral
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "tools/call outputs should align with declared outputSchema when provided";
        let tools: Vec<Tool> = match &ctx.tools {
            Some(v) => v.clone(),
            None => match ctx.session.list_tools().await {
                Ok(v) => {
                    ctx.tools = Some(v.clone());
                    v
                }
                Err(e) => {
                    return vec![CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("Failed to list tools: {e}"),
                    )];
                }
            },
        };

        let with_output: Vec<_> = tools.iter().filter(|t| t.output_schema.is_some()).collect();
        if with_output.is_empty() {
            return vec![
                CheckResult::pass(self.id(), self.name(), self.category(), desc).with_evidence(
                    serde_json::json!({
                        "checked_tools": 0,
                        "skipped_reason": "no outputSchema"
                    }),
                ),
            ];
        }

        let mut findings = Vec::new();
        for tool in with_output {
            let args = build_minimal_args(&tool.input_schema);
            match ctx.session.call_tool(&tool.name, args).await {
                Ok(call_result) => {
                    let out_val = match serde_json::to_value(&call_result) {
                        Ok(v) => v,
                        Err(e) => {
                            findings.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                format!("Tool {} output serialization failed: {e}", tool.name),
                            ));
                            continue;
                        }
                    };
                    let schema = tool.output_schema.as_ref().expect("filtered above");
                    let compiled = match jsonschema::JSONSchema::compile(schema) {
                        Ok(c) => c,
                        Err(e) => {
                            findings.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::Medium,
                                desc,
                                format!("Tool {} outputSchema invalid: {e}", tool.name),
                            ));
                            continue;
                        }
                    };

                    let details: Vec<String> = match compiled.validate(&out_val) {
                        Ok(_) => Vec::new(),
                        Err(errors) => errors.take(3).map(|e| e.to_string()).collect(),
                    };
                    if !details.is_empty() {
                        findings.push(
                            CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                format!("Tool {} output does not match outputSchema", tool.name),
                            )
                            .with_evidence(serde_json::json!({
                                "tool": tool.name,
                                "validation_errors": details
                            })),
                        );
                    }
                }
                Err(e) => {
                    findings.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!(
                            "Tool {} call failed for schema alignment probe: {e}",
                            tool.name
                        ),
                    ));
                }
            }
        }

        if findings.is_empty() {
            vec![CheckResult::pass(
                self.id(),
                self.name(),
                self.category(),
                desc,
            )]
        } else {
            findings
        }
    }
}
