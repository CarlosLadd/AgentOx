//! CONF-002: Validates JSON-RPC 2.0 message structure on responses.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct JsonRpcStructure;

#[async_trait::async_trait]
impl Check for JsonRpcStructure {
    fn id(&self) -> &str {
        "CONF-002"
    }

    fn name(&self) -> &str {
        "JSON-RPC 2.0 message structure"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc =
            "All responses must have jsonrpc=\"2.0\", matching id, and exactly one of result/error";

        // Send a simple tools/list request and inspect the raw response
        let req = JsonRpcRequest::new(9999, "tools/list", Some(serde_json::json!({})));
        let raw = serde_json::to_string(&req).unwrap();

        match ctx.session.send_raw(&raw).await {
            Ok(Some(response_str)) => {
                match serde_json::from_str::<serde_json::Value>(&response_str) {
                    Ok(val) => {
                        let mut results = Vec::new();

                        // Check jsonrpc field
                        match val.get("jsonrpc").and_then(|v| v.as_str()) {
                            Some("2.0") => {}
                            Some(other) => {
                                results.push(
                                    CheckResult::fail(
                                        self.id(),
                                        self.name(),
                                        self.category(),
                                        Severity::High,
                                        desc,
                                        format!("jsonrpc field is \"{other}\" instead of \"2.0\""),
                                    )
                                    .with_evidence(val.clone()),
                                );
                            }
                            None => {
                                results.push(
                                    CheckResult::fail(
                                        self.id(),
                                        self.name(),
                                        self.category(),
                                        Severity::High,
                                        desc,
                                        "jsonrpc field is missing from response",
                                    )
                                    .with_evidence(val.clone()),
                                );
                            }
                        }

                        // Check id matches
                        match val.get("id") {
                            Some(id) if id.as_i64() == Some(9999) => {}
                            Some(id) => {
                                results.push(
                                    CheckResult::fail(
                                        self.id(),
                                        self.name(),
                                        self.category(),
                                        Severity::High,
                                        desc,
                                        format!(
                                            "Response id ({id}) does not match request id (9999)"
                                        ),
                                    )
                                    .with_evidence(val.clone()),
                                );
                            }
                            None => {
                                results.push(
                                    CheckResult::fail(
                                        self.id(),
                                        self.name(),
                                        self.category(),
                                        Severity::High,
                                        desc,
                                        "Response is missing id field",
                                    )
                                    .with_evidence(val.clone()),
                                );
                            }
                        }

                        // Check exactly one of result/error
                        let has_result = val.get("result").is_some();
                        let has_error = val.get("error").is_some();
                        if has_result && has_error {
                            results.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                "Response has both result and error (must have exactly one)",
                            ));
                        } else if !has_result && !has_error {
                            results.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                "Response has neither result nor error (must have exactly one)",
                            ));
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
                    Err(e) => {
                        vec![CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::High,
                            desc,
                            format!("Response is not valid JSON: {e}"),
                        )]
                    }
                }
            }
            Ok(None) => {
                vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    "No response received from server",
                )]
            }
            Err(e) => {
                vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    format!("Transport error: {e}"),
                )]
            }
        }
    }
}
