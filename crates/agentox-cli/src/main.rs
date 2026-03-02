use agentox_core::{
    checks::runner::{CheckContext, CheckRunner},
    client::{session::McpSession, stdio::StdioTransport},
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
            no_color,
        } => {
            if stdio.is_none() && target.is_none() {
                anyhow::bail!(
                    "Either --stdio or --target must be specified.\n\n\
                     Examples:\n  \
                     agentox audit --stdio \"npx -y @modelcontextprotocol/server-filesystem /tmp\"\n  \
                     agentox audit --target http://localhost:8080"
                );
            }

            if matches!(only, Some(CheckCategoryFilter::Behavioral)) {
                anyhow::bail!("Behavioral checks are not implemented yet.");
            }

            if target.is_some() {
                anyhow::bail!("HTTP/SSE transport is not yet implemented. Use --stdio for now.");
            }

            // Respect --no-color and the NO_COLOR env var
            if no_color || std::env::var("NO_COLOR").is_ok() {
                colored::control::set_override(false);
            }

            let command = stdio.unwrap();

            eprintln!("{}", "AgentOx".bold().cyan());
            eprintln!("{} v{}", "Version".dimmed(), env!("CARGO_PKG_VERSION"));
            eprintln!("{} {}", "Target".dimmed(), command);
            eprintln!();

            // --- Connect and initialize ---
            let mut transport = StdioTransport::spawn(&command)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;
            let request_timeout = std::time::Duration::from_secs(timeout.max(1));
            transport.set_read_timeout(request_timeout);

            let mut session = McpSession::new(Box::new(transport));
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
            let mut ctx = CheckContext::new(session, command.clone());
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
                Some(CheckCategoryFilter::Behavioral) => unreachable!(),
                None => runner.register_default_v0_2_checks(),
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
            let server_info = ctx.init_result.map(|i| i.server_info);

            let report = AuditReport::from_results(
                all_results,
                command,
                server_info,
                protocol_version,
                duration_ms,
            );

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

            // Non-zero exit code when findings exist — enables CI fail gates
            if report.summary.failed > 0 {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
