//! CONF-007: Error responses use spec-defined error codes.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct ErrorCodeCorrectness;

#[async_trait::async_trait]
impl Check for ErrorCodeCorrectness {
    fn id(&self) -> &str {
        "CONF-007"
    }

    fn name(&self) -> &str {
        "Error code correctness"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Error responses must use standard JSON-RPC error codes";
        let mut results = Vec::new();

        // Test: call a nonexistent tool — should get an error
        let req = JsonRpcRequest::new(
            ctx.session.next_id(),
            "tools/call",
            Some(serde_json::json!({
                "name": "__agentox_nonexistent_tool__",
                "arguments": {}
            })),
        );

        match ctx.session.send_request(&req).await {
            Ok(response) => {
                if let Some(error) = &response.error {
                    // Standard JSON-RPC error codes
                    let standard_codes = [
                        -32700, // Parse Error
                        -32600, // Invalid Request
                        -32601, // Method Not Found
                        -32602, // Invalid Params
                        -32603, // Internal Error
                    ];
                    // MCP-specific codes (range -32000 to -32099)
                    let is_standard = standard_codes.contains(&error.code);
                    let is_mcp_range = (-32099..=-32000).contains(&error.code);

                    if !is_standard && !is_mcp_range {
                        results.push(CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::Medium,
                            desc,
                            format!(
                                "Non-standard error code {} for nonexistent tool call (expected standard JSON-RPC or MCP code)",
                                error.code
                            ),
                        ));
                    }
                }
                // If no error, the server accepted a nonexistent tool — that's not this check's concern
            }
            Err(e) => {
                results.push(CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Medium,
                    desc,
                    format!("Transport error during error code test: {e}"),
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
