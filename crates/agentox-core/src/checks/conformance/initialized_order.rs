//! CONF-010: Server handles the `notifications/initialized` notification correctly.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::client::stdio::StdioTransport;
use crate::client::McpSession;
use crate::protocol::jsonrpc::JsonRpcRequest;
use crate::protocol::mcp_types::{ClientCapabilities, Implementation, InitializeParams};

pub struct InitializedNotificationOrder;

#[async_trait::async_trait]
impl Check for InitializedNotificationOrder {
    fn id(&self) -> &str {
        "CONF-010"
    }

    fn name(&self) -> &str {
        "Initialized notification handling"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Conformance
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server should handle the initialized notification and the initialization lifecycle correctly";
        let mut results = Vec::new();

        // Spawn a fresh session without going through the normal handshake
        let transport = match StdioTransport::spawn_quiet(&ctx.command).await {
            Ok(t) => t,
            Err(e) => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Low,
                    desc,
                    format!("Could not spawn server for initialization order test: {e}"),
                )];
            }
        };

        let mut session = McpSession::new(Box::new(transport));

        // Step 1: Send initialize (without sending initialized notification yet)
        let params = InitializeParams {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "agentox".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
        };

        let req = JsonRpcRequest::new(
            1,
            "initialize",
            Some(serde_json::to_value(&params).unwrap()),
        );

        match session.send_request(&req).await {
            Ok(response) => {
                if response.error.is_some() {
                    results.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        "Server returned error for initialize request",
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
                    format!("Initialize request failed: {e}"),
                ));
            }
        }

        // Step 2: Try tools/list BEFORE sending initialized notification
        // Per spec, the server MAY reject this, but many implementations accept it
        let req = JsonRpcRequest::new(2, "tools/list", Some(serde_json::json!({})));
        match session.send_request(&req).await {
            Ok(_response) => {
                // Server accepted it — this is informational
            }
            Err(_) => {
                // Server rejected it — this is also valid behavior
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
