//! SEC-002: Validate handling of malformed `tools/call` parameters.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct ToolParameterBoundaryValidation;

fn accepted_error_codes_for_case(case: &str) -> &'static [i64] {
    match case {
        "missing name" => &[-32600, -32601, -32602],
        "name is wrong type" => &[-32600, -32601, -32602],
        "unknown tool" => &[-32601, -32602],
        "arguments wrong type" => &[-32600, -32601, -32602, -32000],
        _ => &[-32600, -32601, -32602],
    }
}

#[async_trait::async_trait]
impl Check for ToolParameterBoundaryValidation {
    fn id(&self) -> &str {
        "SEC-002"
    }

    fn name(&self) -> &str {
        "Tool parameter boundary validation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Security
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Malformed tools/call parameters should return structured JSON-RPC errors";

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
                        format!("Failed to list tools before parameter validation tests: {e}"),
                    )];
                }
            },
        };

        let known_tool = tools.first().map(|t| t.name.clone());
        let mut cases = vec![
            (
                "missing name",
                serde_json::json!({ "arguments": { "message": "x" } }),
            ),
            (
                "name is wrong type",
                serde_json::json!({ "name": 123, "arguments": {} }),
            ),
            (
                "unknown tool",
                serde_json::json!({ "name": "__missing__", "arguments": {} }),
            ),
        ];
        if let Some(tool_name) = known_tool {
            cases.push((
                "arguments wrong type",
                serde_json::json!({ "name": tool_name, "arguments": "not-an-object" }),
            ));
        }

        let mut findings = Vec::new();
        for (label, params) in cases {
            let req =
                JsonRpcRequest::new(ctx.session.next_id(), "tools/call", Some(params.clone()));
            match ctx.session.send_request(&req).await {
                Ok(resp) => {
                    if let Some(error) = resp.error {
                        let accepted = accepted_error_codes_for_case(label);
                        if !accepted.contains(&error.code) {
                            findings.push(
                                CheckResult::fail(
                                    self.id(),
                                    self.name(),
                                    self.category(),
                                    Severity::Medium,
                                    desc,
                                    format!(
                                        "{label}: expected one of {:?}, got code {}",
                                        accepted, error.code
                                    ),
                                )
                                .with_evidence(serde_json::json!({
                                    "case": label,
                                    "params": params,
                                    "error_code": error.code,
                                    "error_message": error.message
                                })),
                            );
                        }
                    } else if resp
                        .result
                        .as_ref()
                        .and_then(|v| v.get("isError"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        // Some servers return structured tool errors through result payloads.
                    } else {
                        findings.push(
                            CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                format!("{label}: server accepted malformed tool-call parameters"),
                            )
                            .with_evidence(serde_json::json!({
                                "case": label,
                                "params": params,
                                "response_result": resp.result
                            })),
                        );
                    }
                }
                Err(e) => {
                    findings.push(
                        CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::Critical,
                            desc,
                            format!("{label}: transport/session failed while testing parameter boundary: {e}"),
                        )
                        .with_evidence(serde_json::json!({
                            "case": label,
                            "params": params
                        })),
                    );
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
