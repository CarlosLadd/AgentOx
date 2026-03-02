//! SEC-003: Detect sensitive details leaking through error messages.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::jsonrpc::JsonRpcRequest;

pub struct ErrorLeakageDetection;

const LEAK_PATTERNS: &[&str] = &[
    "/users/",
    "/home/",
    "traceback",
    "stack trace",
    "panic at",
    "exception:",
    "aws_secret_access_key",
    "api_key",
    "bearer ",
];

#[async_trait::async_trait]
impl Check for ErrorLeakageDetection {
    fn id(&self) -> &str {
        "SEC-003"
    }

    fn name(&self) -> &str {
        "Error leakage detection"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Security
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Error responses should avoid leaking internal paths, traces, or secrets";

        let probes = vec![
            (
                "unknown method",
                JsonRpcRequest::new(
                    ctx.session.next_id(),
                    "internal/debug_dump",
                    Some(serde_json::json!({})),
                ),
            ),
            (
                "malformed tools/call",
                JsonRpcRequest::new(
                    ctx.session.next_id(),
                    "tools/call",
                    Some(serde_json::json!({ "name": 123, "arguments": {} })),
                ),
            ),
        ];

        let mut findings = Vec::new();
        for (label, req) in probes {
            match ctx.session.send_request(&req).await {
                Ok(resp) => {
                    if let Some(error) = resp.error {
                        let mut haystack = error.message;
                        if let Some(data) = error.data {
                            haystack.push('\n');
                            haystack.push_str(&data.to_string());
                        }
                        let lower = haystack.to_ascii_lowercase();
                        if let Some(pattern) = LEAK_PATTERNS.iter().find(|p| lower.contains(**p)) {
                            findings.push(
                                CheckResult::fail(
                                    self.id(),
                                    self.name(),
                                    self.category(),
                                    Severity::Medium,
                                    desc,
                                    format!("{label}: error content matched sensitive pattern \"{pattern}\""),
                                )
                                .with_evidence(serde_json::json!({
                                    "probe": label,
                                    "pattern": pattern,
                                    "error_excerpt": lower.chars().take(240).collect::<String>(),
                                })),
                            );
                        }
                    }
                }
                Err(e) => {
                    findings.push(CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("{label}: failed to retrieve error response for leakage test: {e}"),
                    ));
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
