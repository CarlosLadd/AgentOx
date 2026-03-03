//! SEC-004: Lightweight resource-exhaustion resilience probe.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::security::constants::exhaustion_probe_profile;
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;
use std::time::Instant;

pub struct ResourceExhaustionGuardrail;

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
        let profile = exhaustion_probe_profile(ctx.request_timeout);
        let timeout_ms = ctx.request_timeout.as_millis();

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

        let large = "A".repeat(profile.large_arg_bytes);
        let mut max_latency_ms = 0_u128;

        for i in 0..profile.burst_requests {
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
                        "burst_requests": profile.burst_requests,
                        "arg_bytes": profile.large_arg_bytes,
                        "timeout_ms": timeout_ms
                    }))];
                }
            }
        }

        if max_latency_ms > timeout_ms + 50 {
            let _ = session.shutdown().await;
            return vec![CheckResult::fail(
                self.id(),
                self.name(),
                self.category(),
                Severity::Medium,
                desc,
                format!(
                    "Latency exceeded deterministic timeout budget: max={}ms budget={}ms",
                    max_latency_ms, timeout_ms
                ),
            )
            .with_evidence(serde_json::json!({
                "burst_requests": profile.burst_requests,
                "arg_bytes": profile.large_arg_bytes,
                "max_latency_ms": max_latency_ms,
                "timeout_ms": timeout_ms
            }))];
        }

        // Final liveness check.
        let ping = JsonRpcRequest::new(90_001, "tools/list", Some(serde_json::json!({})));
        let result = match session.send_request(&ping).await {
            Ok(_) => CheckResult::pass(self.id(), self.name(), self.category(), desc)
                .with_evidence(serde_json::json!({
                    "burst_requests": profile.burst_requests,
                    "arg_bytes": profile.large_arg_bytes,
                    "max_latency_ms": max_latency_ms,
                    "timeout_ms": timeout_ms
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
                "burst_requests": profile.burst_requests,
                "arg_bytes": profile.large_arg_bytes,
                "timeout_ms": timeout_ms
            })),
        };

        let _ = session.shutdown().await;
        vec![result]
    }
}
