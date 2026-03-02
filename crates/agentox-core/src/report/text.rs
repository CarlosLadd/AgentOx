//! Colored terminal text report renderer.

use crate::checks::types::Severity;
use crate::report::types::AuditReport;
use colored::*;

/// Render an audit report as colored terminal text.
pub fn render(report: &AuditReport) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!("\n{}\n", "AgentOx Audit Report".bold()));
    out.push_str(&format!("Target: {}\n", report.target));
    if let Some(info) = &report.server_info {
        out.push_str(&format!(
            "Server: {} v{}\n",
            info.name,
            info.version.as_deref().unwrap_or("?")
        ));
    }
    out.push_str(&format!(
        "Protocol: {}\n\n",
        report.protocol_version.as_deref().unwrap_or("?")
    ));

    // Results
    for result in &report.results {
        let badge = if result.passed {
            format!("[{}]", "PASS".green())
        } else {
            match result.severity {
                Severity::Critical => format!("[{}]", "CRIT".red().bold()),
                Severity::High => format!("[{}]", "HIGH".red()),
                Severity::Medium => format!("[{}]", " MED".yellow()),
                Severity::Low => format!("[{}]", " LOW".blue()),
                Severity::Info => format!("[{}]", "INFO".dimmed()),
                Severity::Pass => format!("[{}]", "PASS".green()),
            }
        };
        out.push_str(&format!(
            "{} {} {}\n",
            badge,
            result.check_id.dimmed(),
            result.name
        ));
        if !result.passed {
            out.push_str(&format!("      {}\n", result.message));
        }
    }

    // Summary
    out.push_str(&format!("\n{}\n", "Summary".bold()));
    out.push_str(&format!(
        "  Total: {}, Passed: {}, Failed: {}\n",
        report.summary.total_checks,
        report.summary.passed.to_string().green(),
        if report.summary.failed > 0 {
            report.summary.failed.to_string().red().to_string()
        } else {
            "0".to_string()
        }
    ));
    out.push_str(&format!("  Duration: {}ms\n", report.summary.duration_ms));

    out
}
