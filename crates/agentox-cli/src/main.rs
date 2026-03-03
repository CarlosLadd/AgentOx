use agentox_core::{
    checks::runner::{CheckContext, CheckRunner, ConnectionTarget},
    client::{AgentSession, HttpSseTransport, StdioTransport},
    platform::{A2aProtocolAdapter, AgentProtocol, McpProtocolAdapter, OpenAiToolUseAdapter},
    policy,
    report::{json, text, types::AuditReport},
};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;

/// AgentOx — MCP security & conformance auditor, forged in Rust.
#[derive(Parser)]
#[command(
    name = "agentox",
    version,
    about = "Audit MCP servers for protocol conformance, security, and behavior",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CheckCategoryFilter {
    Conformance,
    Security,
    Behavioral,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum ProtocolSelection {
    Mcp,
    A2a,
    OpenaiToolUse,
}

impl From<ProtocolSelection> for AgentProtocol {
    fn from(value: ProtocolSelection) -> Self {
        match value {
            ProtocolSelection::Mcp => AgentProtocol::Mcp,
            ProtocolSelection::A2a => AgentProtocol::A2a,
            ProtocolSelection::OpenaiToolUse => AgentProtocol::OpenAiToolUse,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Audit an MCP server for protocol conformance and security issues
    Audit {
        /// Server command for stdio transport
        /// (e.g., "npx -y @modelcontextprotocol/server-filesystem /tmp")
        #[arg(long, value_name = "COMMAND")]
        stdio: Option<String>,

        /// Server HTTP/SSE endpoint URL
        #[arg(long, value_name = "URL")]
        target: Option<String>,

        /// Output format: text (default), json
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        format: OutputFormat,

        /// Run only specific check categories: conformance, security, behavioral
        #[arg(long, value_name = "CATEGORY")]
        only: Option<CheckCategoryFilter>,

        /// Per-check timeout in seconds
        #[arg(long, default_value = "30", value_name = "SECONDS")]
        timeout: u64,

        /// Protocol adapter to use: mcp, a2a, openai-tool-use
        #[arg(long, default_value = "mcp", value_name = "PROTOCOL")]
        protocol: ProtocolSelection,

        /// Policy bundle path (YAML)
        #[arg(long, value_name = "FILE")]
        policy: Option<String>,

        /// Baseline report path for regression comparison
        #[arg(long, value_name = "REPORT_JSON")]
        baseline: Option<String>,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr) // Logs go to stderr; report goes to stdout
        .init();

    match cli.command {
        Commands::Audit {
            stdio,
            target,
            format,
            only,
            timeout,
            protocol,
            policy,
            baseline,
            no_color,
        } => {
            if stdio.is_none() && target.is_none() {
                anyhow::bail!(
                    "Exactly one of --stdio or --target must be specified.\n\n\
                     Examples:\n  \
                     agentox audit --stdio \"npx -y @modelcontextprotocol/server-filesystem /tmp\"\n  \
                     agentox audit --target http://localhost:8080"
                );
            }

            if stdio.is_some() && target.is_some() {
                anyhow::bail!("Use either --stdio or --target, but not both.");
            }

            // Respect --no-color and the NO_COLOR env var
            if no_color || std::env::var("NO_COLOR").is_ok() {
                colored::control::set_override(false);
            }

            let request_timeout = std::time::Duration::from_secs(timeout.max(1));
            let selected_protocol: AgentProtocol = protocol.into();
            let (target_label, mut session, conn_target) = match (stdio, target) {
                (Some(command), None) => {
                    let mut transport = StdioTransport::spawn(&command)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;
                    transport.set_read_timeout(request_timeout);
                    let mcp_session =
                        agentox_core::client::session::McpSession::new(Box::new(transport));
                    let adapter: Box<dyn agentox_core::platform::ProtocolAdapter> =
                        match selected_protocol {
                            AgentProtocol::Mcp => Box::new(McpProtocolAdapter::new(mcp_session)),
                            AgentProtocol::A2a => Box::new(A2aProtocolAdapter::new(mcp_session)),
                            AgentProtocol::OpenAiToolUse => {
                                Box::new(OpenAiToolUseAdapter::new(mcp_session))
                            }
                        };
                    (
                        command.clone(),
                        AgentSession::new(adapter),
                        ConnectionTarget::Stdio {
                            command,
                            protocol: selected_protocol,
                        },
                    )
                }
                (None, Some(endpoint)) => {
                    let transport = HttpSseTransport::new(endpoint.clone(), request_timeout);
                    let mcp_session =
                        agentox_core::client::session::McpSession::new(Box::new(transport));
                    let adapter: Box<dyn agentox_core::platform::ProtocolAdapter> =
                        match selected_protocol {
                            AgentProtocol::Mcp => Box::new(McpProtocolAdapter::new(mcp_session)),
                            AgentProtocol::A2a => Box::new(A2aProtocolAdapter::new(mcp_session)),
                            AgentProtocol::OpenAiToolUse => {
                                Box::new(OpenAiToolUseAdapter::new(mcp_session))
                            }
                        };
                    (
                        endpoint.clone(),
                        AgentSession::new(adapter),
                        ConnectionTarget::HttpSse {
                            endpoint,
                            protocol: selected_protocol,
                        },
                    )
                }
                _ => unreachable!(),
            };

            eprintln!("{}", "AgentOx".bold().cyan());
            eprintln!("{} v{}", "Version".dimmed(), env!("CARGO_PKG_VERSION"));
            eprintln!("{} {}", "Target".dimmed(), target_label);
            eprintln!();

            // --- Connect and initialize ---
            let init_result = session
                .initialize()
                .await
                .map_err(|e| anyhow::anyhow!("MCP handshake failed: {e}"))?;

            eprintln!(
                "{} {} v{}  (protocol {})",
                "Server".dimmed(),
                init_result.server_info.name.bold(),
                init_result.server_info.version.as_deref().unwrap_or("?"),
                init_result.protocol_version.dimmed()
            );

            // --- Build check context ---
            let mut ctx = CheckContext::new(session, conn_target);
            ctx.init_result = Some(init_result.clone());
            ctx.request_timeout = request_timeout;

            // Pre-fetch tools list so checks can share it
            match ctx.session.list_tools().await {
                Ok(tools) => {
                    eprintln!("{} {}", "Tools".dimmed(), tools.len());
                    ctx.tools = Some(tools);
                }
                Err(e) => {
                    tracing::warn!("Could not prefetch tools list: {e}");
                }
            }
            eprintln!();

            // --- Register checks ---
            let mut runner = CheckRunner::new();
            match only {
                Some(CheckCategoryFilter::Conformance) => runner.register_conformance_checks(),
                Some(CheckCategoryFilter::Security) => runner.register_security_checks(),
                Some(CheckCategoryFilter::Behavioral) => runner.register_behavioral_checks(),
                None => runner.register_default_v0_4_checks(),
            }

            let total_checks = runner.check_count();

            // --- Progress bar ---
            let pb = ProgressBar::new(total_checks as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.cyan} [{bar:30.blue/dim}] {pos}/{len}  {msg}")
                    .unwrap()
                    .progress_chars("━╸─"),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));

            let audit_start = Instant::now();

            // --- Run checks with live progress ---
            let all_results = runner
                .run_all_with_progress(&mut ctx, |check_id, check_name, results| {
                    pb.inc(1);

                    let all_passed = results.iter().all(|r| r.passed);
                    let badge = if all_passed {
                        "PASS".green().bold().to_string()
                    } else {
                        "FAIL".red().bold().to_string()
                    };

                    // Clear the progress bar line and print the result above it
                    pb.suspend(|| {
                        eprintln!("  [{badge}] {check_id} {check_name}");
                    });

                    // Update the progress message with the next check hint
                    let completed = pb.position();
                    if completed < total_checks as u64 {
                        pb.set_message("running…".dimmed().to_string());
                    } else {
                        pb.set_message("done".green().to_string());
                    }
                })
                .await;

            let duration_ms = audit_start.elapsed().as_millis() as u64;

            pb.finish_and_clear();
            eprintln!();

            // --- Build report ---
            let protocol_version = ctx.session.protocol_version().map(|s| s.to_string());
            let server_info = ctx.init_result.as_ref().map(|i| i.server_info.clone());

            let mut report = AuditReport::from_results(
                all_results,
                target_label,
                server_info,
                protocol_version,
                duration_ms,
            )
            .with_protocol_metadata(
                ctx.protocol(),
                ctx.session.adapter_metadata(),
                Vec::new(),
            );

            if let Some(policy_path) = policy {
                let bundle = policy::load_policy_file(&policy_path)?;
                let env_name = std::env::var("AGENTOX_ENV").ok();
                let decision = policy::evaluate_report(&report, &bundle, env_name.as_deref());
                report = report.with_policy_decision(decision);
            }

            if let Some(baseline_path) = baseline {
                let baseline_raw = std::fs::read_to_string(&baseline_path).map_err(|e| {
                    anyhow::anyhow!("Failed reading baseline report {}: {e}", baseline_path)
                })?;
                let baseline_report: AuditReport =
                    serde_json::from_str(&baseline_raw).map_err(|e| {
                        anyhow::anyhow!("Failed parsing baseline report {}: {e}", baseline_path)
                    })?;
                let delta = policy::compare_with_baseline(&report, &baseline_report);
                if !delta.new_high_or_critical.is_empty() {
                    let mut reasons = vec![format!(
                        "New HIGH/CRITICAL regressions vs baseline: {}",
                        delta.new_high_or_critical.join(", ")
                    )];
                    if !delta.new_failed_checks.is_empty() {
                        reasons.push(format!(
                            "New failed checks: {}",
                            delta.new_failed_checks.join(", ")
                        ));
                    }
                    report =
                        report.with_policy_decision(agentox_core::report::types::PolicyDecision {
                            status: agentox_core::report::types::PolicyDecisionStatus::Fail,
                            reasons,
                        });
                }
            }

            if let Ok(sig) = json::evidence_signature(&report) {
                report = report.with_evidence_signature(sig);
            }

            // --- Shut down session ---
            let _ = ctx.session.shutdown().await;

            // --- Print summary to stderr ---
            let passed_str = format!("{} passed", report.summary.passed).green();
            let failed_str = if report.summary.failed > 0 {
                format!("{} failed", report.summary.failed).red().bold()
            } else {
                format!("{} failed", report.summary.failed).green()
            };
            let duration_str = if duration_ms < 1000 {
                format!("{duration_ms}ms")
            } else {
                format!("{:.1}s", duration_ms as f64 / 1000.0)
            };
            eprintln!(
                "  {} {}, {} ({duration_str})",
                "Summary".bold(),
                passed_str,
                failed_str,
            );
            eprintln!();

            // --- Render output to stdout ---
            match format {
                OutputFormat::Json => {
                    let json_out = json::render(&report)
                        .map_err(|e| anyhow::anyhow!("JSON serialization failed: {e}"))?;
                    println!("{json_out}");
                }
                OutputFormat::Text => {
                    let text_out = text::render(&report);
                    println!("{text_out}");
                }
            }

            // Non-zero exit code when findings exist or policy gates fail.
            let policy_failed = report.policy_decision.as_ref().is_some_and(|decision| {
                matches!(
                    decision.status,
                    agentox_core::report::types::PolicyDecisionStatus::Fail
                )
            });
            if report.summary.failed > 0 || policy_failed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
