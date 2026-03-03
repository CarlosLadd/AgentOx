//! SEC-001: Probe tool-call behavior with prompt-injection strings.

use crate::checks::runner::{Check, CheckContext};
use crate::checks::security::constants::{
    truncate_for_evidence, EVIDENCE_MAX_CHARS, INJECTION_LEAK_MARKERS, INJECTION_PAYLOADS,
};
use crate::checks::types::{CheckCategory, CheckResult, Severity};
use crate::protocol::mcp_types::Content;

pub struct PromptInjectionEchoSafety;

#[async_trait::async_trait]
impl Check for PromptInjectionEchoSafety {
    fn id(&self) -> &str {
        "SEC-001"
    }

    fn name(&self) -> &str {
        "Prompt-injection echo safety"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::Security
    }

    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let desc = "Tool calls should reject or safely handle prompt-injection-like input";

        let tools = match &ctx.tools {
            Some(tools) => tools.clone(),
            None => match ctx.session.list_tools().await {
                Ok(tools) => {
                    ctx.tools = Some(tools.clone());
                    tools
                }
                Err(e) => {
                    return vec![CheckResult::fail(
                        self.id(),
                        self.name(),
                        self.category(),
                        Severity::High,
                        desc,
                        format!("Failed to list tools for security probing: {e}"),
                    )];
                }
            },
        };

        if tools.is_empty() {
            let mut pass = CheckResult::pass(self.id(), self.name(), self.category(), desc);
            pass.message = "No tools exposed; injection probe is not applicable".to_string();
            return vec![pass.with_evidence(serde_json::json!({ "tools": 0 }))];
        }

        let mut findings = Vec::new();
        let mut benign_echoes = 0usize;
        for tool in &tools {
            for payload in INJECTION_PAYLOADS {
                let args = serde_json::json!({
                    "message": payload,
                    "text": payload,
                    "name": payload
                });

                match ctx.session.call_tool(&tool.name, args).await {
                    Ok(result) => {
                        let text_out: String = result
                            .content
                            .iter()
                            .filter_map(|c| match c {
                                Content::Text { text } => Some(text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        let lowered = text_out.to_ascii_lowercase();
                        let leaked = INJECTION_LEAK_MARKERS
                            .iter()
                            .any(|marker| lowered.contains(marker));
                        if leaked {
                            findings.push(
                                CheckResult::fail(
                                    self.id(),
                                    self.name(),
                                    self.category(),
                                    Severity::High,
                                    desc,
                                    format!(
                                        "Tool \"{}\" returned suspicious content for injection probe",
                                        tool.name
                                    ),
                                )
                                .with_evidence(serde_json::json!({
                                    "tool": tool.name,
                                    "payload": payload,
                                    "output_sample": truncate_for_evidence(&text_out, EVIDENCE_MAX_CHARS),
                                })),
                            );
                        } else if text_out.trim() == *payload {
                            // Explicitly treat a pure echo as benign if there is no sensitive marker leakage.
                            benign_echoes += 1;
                        }
                    }
                    Err(crate::error::SessionError::JsonRpc { .. }) => {
                        // Rejected requests are acceptable and often safer.
                    }
                    Err(e) => {
                        findings.push(
                            CheckResult::fail(
                                self.id(),
                                self.name(),
                                self.category(),
                                Severity::High,
                                desc,
                                format!(
                                    "Tool \"{}\" probe failed with transport/session error: {e}",
                                    tool.name
                                ),
                            )
                            .with_evidence(serde_json::json!({
                                "tool": tool.name,
                                "payload": payload
                            })),
                        );
                    }
                }
            }
        }

        if findings.is_empty() {
            vec![
                CheckResult::pass(self.id(), self.name(), self.category(), desc).with_evidence(
                    serde_json::json!({
                        "tools_probed": tools.len(),
                        "payloads_per_tool": INJECTION_PAYLOADS.len(),
                        "benign_echo_responses": benign_echoes
                    }),
                ),
            ]
        } else {
            findings
        }
    }
}
