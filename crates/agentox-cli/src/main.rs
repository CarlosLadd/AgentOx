use agentox_core::{
    checks::runner::{CheckContext, CheckRunner},
    client::{session::McpSession, stdio::StdioTransport},
    report::{json, text, types::AuditReport},
};
use clap::{Parser, Subcommand};
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
        format: String,

        /// Run only specific check categories: conformance, security, behavioral
        #[arg(long, value_name = "CATEGORY")]
        only: Option<String>,

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
            timeout: _timeout,
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

            if target.is_some() {
                anyhow::bail!("HTTP/SSE transport is not yet implemented. Use --stdio for now.");
            }

            // Respect --no-color and the NO_COLOR env var
            if no_color || std::env::var("NO_COLOR").is_ok() {
                colored::control::set_override(false);
            }

            let command = stdio.unwrap();

            eprintln!("AgentOx v{}", env!("CARGO_PKG_VERSION"));
            eprintln!("Target: {command}");
            eprintln!();

            // --- Connect and initialize ---
            let transport = StdioTransport::spawn(&command)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;

            let mut session = McpSession::new(Box::new(transport));
            let init_result = session
                .initialize()
                .await
                .map_err(|e| anyhow::anyhow!("MCP handshake failed: {e}"))?;

            eprintln!(
                "Connected: {} v{}  (protocol {})",
                init_result.server_info.name,
                init_result.server_info.version.as_deref().unwrap_or("?"),
                init_result.protocol_version
            );

            // --- Build check context ---
            let mut ctx = CheckContext::new(session, command.clone());
            ctx.init_result = Some(init_result.clone());

            // Pre-fetch tools list so checks can share it
            match ctx.session.list_tools().await {
                Ok(tools) => {
                    eprintln!("Tools found: {}", tools.len());
                    ctx.tools = Some(tools);
                }
                Err(e) => {
                    tracing::warn!("Could not prefetch tools list: {e}");
                }
            }
            eprintln!();

            // --- Register checks ---
            let mut runner = CheckRunner::new();
            match only.as_deref() {
                Some("security") | Some("behavioral") => {
                    eprintln!("Note: Only conformance checks are available in v0.1.0.");
                    runner.register_conformance_checks();
                }
                _ => runner.register_conformance_checks(),
            }

            let total_checks = runner.check_count();

            // --- Progress bar ---
            let pb = ProgressBar::new(total_checks as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.cyan} [{bar:38.blue/dim}] {pos}/{len} {msg:.dim}")
                    .unwrap()
                    .progress_chars("=> "),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            pb.set_message("running checks…");

            let audit_start = Instant::now();
            let all_results = runner.run_all(&mut ctx).await;
            let duration_ms = audit_start.elapsed().as_millis() as u64;

            pb.finish_and_clear();

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

            // --- Render output ---
            match format.as_str() {
                "json" => {
                    let json_out = json::render(&report)
                        .map_err(|e| anyhow::anyhow!("JSON serialization failed: {e}"))?;
                    println!("{json_out}");
                }
                _ => {
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
