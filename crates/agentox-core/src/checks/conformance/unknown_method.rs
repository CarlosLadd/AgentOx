//! CONF-006: Server returns -32601 (Method Not Found) for unknown methods.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct UnknownMethodHandling;

const UNKNOWN_METHODS: &[&str] = &["nonexistent/method", "tools/nonexistent", "foo"];

#[async_trait::async_trait]
impl Check for UnknownMethodHandling {
    fn id(&self) -> &str {
        "CONF-006"
    }

    fn name(&self) -> &str {
        "Unknown method handling"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server must return JSON-RPC error -32601 for unknown methods";
        let mut results = Vec::new();

        for method in UNKNOWN_METHODS {
            let req =
                JsonRpcRequest::new(ctx.session.next_id(), *method, Some(serde_json::json!({})));

            match ctx.session.send_request(&req).await {
                Ok(response) => {
                    if let Some(error) = &response.error {
                        if error.code != -32601 {
                            results.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::Medium,
                                desc,
                                format!(
                                    "Method \"{method}\": expected error code -32601, got {}",
                                    error.code
                                ),
                            ));
                        }
                    } else {
                        results.push(CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::High,
                            desc,
                            format!(
                                "Method \"{method}\": server returned success instead of error"
                            ),
                        ));
                    }
                }
                Err(e) => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("Method \"{method}\": transport error: {e}"),
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
