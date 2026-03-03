//! BHV-001: tools/list should be idempotent within a session.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::mcp_types::Tool;

pub struct IdempotencyBaseline;

fn fingerprint_tools(tools: &[Tool]) -> Vec<String> {
    let mut out: Vec<String> = tools
        .iter()
        .map(|t| {
            let input = serde_json::to_string(&t.input_schema).unwrap_or_else(|_| "{}".to_string());
            let output = t
                .output_schema
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok())
                .unwrap_or_else(|| "null".to_string());
            format!("{}|{}|{}", t.name, input, output)
        })
        .collect();
    out.sort();
    out
}

#[async_trait::async_trait]
impl Check for IdempotencyBaseline {
    fn id(&self) -> &str {
        "BHV-001"
    }

    fn name(&self) -> &str {
        "Idempotency baseline"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Behavioral
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc =
            "Repeated tools/list calls in a single session should produce a stable tool fingerprint";
        let first = match ctx.session.list_tools().await {
            Ok(v) => v,
            Err(e) => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Medium,
                    desc,
                    format!("First tools/list failed: {e}"),
                )];
            }
        };
        let second = match ctx.session.list_tools().await {
            Ok(v) => v,
            Err(e) => {
                return vec![CheckResult::fail(
                    self.id(),
                    self.name(),
                    self.category(),
                    Severity::Medium,
                    desc,
                    format!("Second tools/list failed: {e}"),
                )];
            }
        };

        let fp1 = fingerprint_tools(&first);
        let fp2 = fingerprint_tools(&second);
        if fp1 == fp2 {
            vec![CheckResult::pass(
                self.id(),
                self.name(),
                self.category(),
                desc,
            )]
        } else {
            let only_1: Vec<_> = fp1.iter().filter(|x| !fp2.contains(*x)).cloned().collect();
            let only_2: Vec<_> = fp2.iter().filter(|x| !fp1.contains(*x)).cloned().collect();
            vec![CheckResult::fail(
                self.id(),
                self.name(),
                self.category(),
                Severity::Medium,
                desc,
                "tools/list fingerprint changed between identical calls",
            )
            .with_evidence(serde_json::json!({
                "fingerprint_1_count": fp1.len(),
                "fingerprint_2_count": fp2.len(),
                "only_in_first": only_1,
                "only_in_second": only_2
            }))]
        }
    }
}
