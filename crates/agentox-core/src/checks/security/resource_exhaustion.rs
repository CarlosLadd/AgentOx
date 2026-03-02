//! SEC-004: Lightweight resource-exhaustion resilience probe.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;
use std::time::Instant;

pub struct ResourceExhaustionGuardrail;

const BURST_REQUESTS: usize = 25;
const LARGE_ARG_BYTES: usize = 16 * 1024;

#[async_trait::async_trait]
impl Check for ResourceExhaustionGuardrail {
    fn id(&self) -> &str {
        "SEC-004"
    }

    fn name(&self) -> &str {
        "Resource-exhaustion guardrail"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Security
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Server should remain responsive under bounded high-volume/large-input probes";

        let mut session = match ctx.disposable_session().await {
            Ok(s) => s,
            Err(e) => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Critical,
                    desc,
                    format!("Could not create disposable session for burst probe: {e}"),
                )];
            }
        };

        let large = "A".repeat(LARGE_ARG_BYTES);
        let mut max_latency_ms = 0_u128;

        for i in 0..BURST_REQUESTS {
            let req = JsonRpcRequest::new(
                50_000 + i as i64,
                "tools/list",
                Some(serde_json::json!({ "cursor": large })),
            );
            let started = Instant::now();
            match session.send_request(&req).await {
                Ok(_) => {
                    max_latency_ms = max_latency_ms.max(started.elapsed().as_millis());
                }
                Err(e) => {
                    let _ = session.shutdown().await;
                    return vec![CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("Server became unstable during bounded burst probe: {e}"),
                    )
                    .with_evidence(serde_json::json!({
                        "request_index": i,
                        "burst_requests": BURST_REQUESTS,
                        "arg_bytes": LARGE_ARG_BYTES
                    }))];
                }
            }
        }

        // Final liveness check.
        let ping = JsonRpcRequest::new(90_001, "tools/list", Some(serde_json::json!({})));
        let result = match session.send_request(&ping).await {
            Ok(_) => CheckResult::pass(self.id(), self.name(), self.category(), desc)
                .with_evidence(serde_json::json!({
                    "burst_requests": BURST_REQUESTS,
                    "arg_bytes": LARGE_ARG_BYTES,
                    "max_latency_ms": max_latency_ms
                })),
            Err(e) => CheckResult::fail(
                self.id(),
                self.name(),
                self.category(),
                Severity::Critical,
                desc,
                format!("Server did not recover after burst probe: {e}"),
            )
            .with_evidence(serde_json::json!({
                "burst_requests": BURST_REQUESTS,
                "arg_bytes": LARGE_ARG_BYTES
            })),
        };

        let _ = session.shutdown().await;
        vec![result]
    }
}
