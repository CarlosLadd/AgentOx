//! BHV-003: Error semantics should be deterministic for repeated malformed requests.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct DeterministicErrorSemantics;

fn classify_message(msg: &str) -> &'static str {
    let lower = msg.to_ascii_lowercase();
    if lower.is_empty() {
        "empty"
    } else if lower.contains("invalid") {
        "invalid"
    } else if lower.contains("method") {
        "method"
    } else if lower.contains("params") {
        "params"
    } else {
        "other"
    }
}

#[async_trait::async_trait]
impl Check for DeterministicErrorSemantics {
    fn id(&self) -> &str {
        "BHV-003"
    }

    fn name(&self) -> &str {
        "Deterministic error semantics"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Behavioral
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Repeated malformed requests should produce stable error semantics";
        let bad_params = serde_json::json!({"arguments": {"foo": "bar"}});
        let req1 = JsonRpcRequest::new(
            ctx.session.next_id(),
            "tools/call",
            Some(bad_params.clone()),
        );
        let req2 = JsonRpcRequest::new(ctx.session.next_id(), "tools/call", Some(bad_params));

        let r1 = ctx.session.send_request(&req1).await;
        let r2 = ctx.session.send_request(&req2).await;

        match (r1, r2) {
            (Ok(a), Ok(b)) => match (a.error, b.error) {
                (Some(e1), Some(e2)) => {
                    let c1 = classify_message(&e1.message);
                    let c2 = classify_message(&e2.message);
                    if e1.code == e2.code && c1 == c2 {
                        vec![CheckResult::pass(
                            self.id(),
                            self.name(),
                            self.category(),
                            desc,
                        )]
                    } else {
                        vec![CheckResult::fail(
                            self.id(),
                            self.name(),
                            self.category(),
                            Severity::Medium,
                            desc,
                            "Malformed-request error semantics changed across repeated calls",
                        )
                        .with_evidence(serde_json::json!({
                            "first": {"code": e1.code, "message": e1.message, "class": c1},
                            "second": {"code": e2.code, "message": e2.message, "class": c2}
                        }))]
                    }
                }
                (None, None) => vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Medium,
                    desc,
                    "Server returned success twice for malformed requests",
                )],
                (left, right) => vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Medium,
                    desc,
                    "One malformed request errored while the other did not",
                )
                .with_evidence(serde_json::json!({
                    "first_has_error": left.is_some(),
                    "second_has_error": right.is_some()
                }))],
            },
            (Err(e1), Err(e2)) => vec![CheckResult::fail(
                self.id(),
                self.name(),
                self.category(),
                Severity::Medium,
                desc,
                "Transport failed for both malformed probes",
            )
            .with_evidence(serde_json::json!({
                "first": e1.to_string(),
                "second": e2.to_string()
            }))],
            (Err(e), Ok(_)) | (Ok(_), Err(e)) => vec![CheckResult::fail(
                self.id(),
                self.name(),
                self.category(),
                Severity::Medium,
                desc,
                "Inconsistent transport behavior between repeated malformed probes",
            )
            .with_evidence(serde_json::json!({ "error": e.to_string() }))],
        }
    }
}
