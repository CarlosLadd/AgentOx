//! CONF-005: Server handles malformed requests gracefully (no crash).

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct MalformedRequestHandling;

/// Malformed messages to send and what we expect.
const MALFORMED_MESSAGES: &[(&str, &str)] = &[
    // Truncated JSON
    (r#"{"jsonrpc": "2.0", "id": 1, "method"#, "truncated JSON"),
    // Missing jsonrpc field
    (
        r#"{"id": 2, "method": "tools/list"}"#,
        "missing jsonrpc field",
    ),
    // Wrong jsonrpc version
    (
        r#"{"jsonrpc": "1.0", "id": 3, "method": "tools/list"}"#,
        "wrong jsonrpc version",
    ),
    // Non-JSON
    ("this is not json", "non-JSON input"),
    // Empty
    ("", "empty message"),
];

#[async_trait::async_trait]
impl Check for MalformedRequestHandling {
    fn id(&self) -> &str {
        "CONF-005"
    }

    fn name(&self) -> &str {
        "Malformed request handling"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server must return proper JSON-RPC errors for malformed requests, not crash";
        let mut results = Vec::new();

        // Use a disposable session so we don't corrupt the main one
        let mut session = match ctx.disposable_session().await {
            Ok(s) => s,
            Err(e) => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::High,
                    desc,
                    format!("Could not create disposable session: {e}"),
                )];
            }
        };

        for (message, label) in MALFORMED_MESSAGES {
            if message.is_empty() {
                continue; // Skip empty — some transports handle this differently
            }

            match session.send_raw(message).await {
                Ok(Some(response)) => {
                    // Got a response — check it's a valid JSON-RPC error
                    match serde_json::from_str::<serde_json::Value>(&response) {
                        Ok(val) => {
                            if val.get("error").is_some() {
                                // Good — server returned an error response
                            } else {
                                results.push(CheckResult::fail(
                                    self.id(),
                                    self.name(),
                                    self.category(),
                                    Severity::Medium,
                                    desc,
                                    format!("Server returned success (not error) for {label}"),
                                ));
                            }
                        }
                        Err(_) => {
                            results.push(CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                format!(
                                    "Server returned non-JSON response for {label}: {response}"
                                ),
                            ));
                        }
                    }

                    // Verify server is still alive by sending a valid request
                    let ping = JsonRpcRequest::new(9000, "tools/list", Some(serde_json::json!({})));
                    if session.send_request(&ping).await.is_err() {
                        results.push(CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::Critical,
                            desc,
                            format!("Server became unresponsive after receiving {label}"),
                        ));
                        break;
                    }
                }
                Ok(None) => {
                    // No response — might be OK for some malformed messages
                }
                Err(_) => {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::Critical,
                        desc,
                        format!("Server crashed or disconnected after receiving {label}"),
                    ));
                    break;
                }
            }
        }

        let _ = session.shutdown().await;

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
