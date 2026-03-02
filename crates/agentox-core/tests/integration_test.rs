//! Integration tests for AgentOx.
//!
//! These tests spawn the mock MCP server binary to exercise the full audit stack.
//! Run with: `cargo test -p agentox-core --test integration_test`
//!
//! The mock server binary is built as part of the workspace. Make sure to run
//! `cargo build --workspace` at least once before running these tests.

use agentox_core::{
    checks::runner::{CheckContext, CheckRunner},
    client::{session::McpSession, stdio::StdioTransport},
    report::types::AuditReport,
};

/// Absolute path to the compiled mock server binary.
fn mock_server_bin() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let root = std::path::Path::new(manifest)
        .join("../..")
        .canonicalize()
        .expect("cannot resolve workspace root");
    root.join("target/debug/mock-mcp-server")
        .to_string_lossy()
        .to_string()
}

/// Build a `CheckContext` connected to a mock server.
/// `env_overrides` are set as environment variables via `/usr/bin/env`.
async fn setup_ctx(env_overrides: &[(&str, &str)]) -> CheckContext {
    let bin = mock_server_bin();

    let shell_cmd = if env_overrides.is_empty() {
        bin
    } else {
        let pairs: Vec<String> = env_overrides
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        format!("/usr/bin/env {} {}", pairs.join(" "), bin)
    };

    let transport = StdioTransport::spawn(&shell_cmd)
        .await
        .unwrap_or_else(|e| panic!("failed to spawn mock server ({shell_cmd}): {e}"));

    let mut session = McpSession::new(Box::new(transport));
    let init_result = session
        .initialize()
        .await
        .expect("failed to initialize MCP session");

    let mut ctx = CheckContext::new(session, shell_cmd);
    ctx.init_result = Some(init_result);

    let tools = ctx.session.list_tools().await.unwrap_or_default();
    ctx.tools = Some(tools);

    ctx
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_conformant_server_passes_all_checks() {
    let mut ctx = setup_ctx(&[]).await;

    let mut runner = CheckRunner::new();
    runner.register_conformance_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(
        failed.is_empty(),
        "Expected 0 failures against conformant mock server, got:\n{:#?}",
        failed
    );
}

#[tokio::test]
async fn test_report_summary_counts_are_consistent() {
    let mut ctx = setup_ctx(&[]).await;

    let mut runner = CheckRunner::new();
    runner.register_conformance_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    let report = AuditReport::from_results(results, "mock-server".to_string(), None, None, 100);

    assert_eq!(report.agentox_version, env!("CARGO_PKG_VERSION"));
    assert!(report.summary.total_checks > 0);
    assert_eq!(
        report.summary.passed + report.summary.failed,
        report.summary.total_checks
    );
}

#[tokio::test]
async fn test_json_report_is_valid_json_with_required_fields() {
    let mut ctx = setup_ctx(&[]).await;

    let mut runner = CheckRunner::new();
    runner.register_conformance_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    let report = AuditReport::from_results(results, "mock-server".to_string(), None, None, 100);

    let json_str = agentox_core::report::json::render(&report).expect("JSON render failed");

    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("output is not valid JSON");

    assert!(parsed.get("summary").is_some(), "missing 'summary'");
    assert!(parsed.get("results").is_some(), "missing 'results'");
    assert!(
        parsed.get("agentox_version").is_some(),
        "missing 'agentox_version'"
    );
}

#[tokio::test]
async fn test_server_with_no_tools_still_passes_init_check() {
    let mut ctx = setup_ctx(&[("MOCK_NO_TOOLS", "1")]).await;

    let mut runner = CheckRunner::new();
    runner.register_conformance_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    let conf001 = results
        .iter()
        .find(|r| r.check_id == "CONF-001")
        .expect("CONF-001 should always be present");

    assert!(conf001.passed, "CONF-001 should pass for no-tools server");
}

#[tokio::test]
async fn test_unknown_method_returns_minus_32601() {
    let mut ctx = setup_ctx(&[]).await;

    use agentox_core::protocol::jsonrpc::JsonRpcRequest;

    let req = JsonRpcRequest::new(999, "totally/nonexistent", Some(serde_json::json!({})));
    let resp = ctx
        .session
        .send_request(&req)
        .await
        .expect("transport should not fail");

    let _ = ctx.session.shutdown().await;

    let error = resp
        .error
        .expect("server must return an error for unknown method");
    assert_eq!(
        error.code, -32601,
        "Expected -32601 (Method Not Found), got {}: {}",
        error.code, error.message
    );
}

#[tokio::test]
async fn test_security_checks_run_with_security_category() {
    let mut ctx = setup_ctx(&[]).await;

    let mut runner = CheckRunner::new();
    runner.register_security_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    assert!(
        !results.is_empty(),
        "Expected at least one security check result"
    );
    assert!(
        results.iter().all(|r| matches!(
            r.category,
            agentox_core::checks::types::CheckCategory::Security
        )),
        "All results should be in the security category"
    );
    assert!(
        results.iter().all(|r| r.check_id.starts_with("SEC-")),
        "All check IDs should use SEC-*"
    );
}

#[tokio::test]
async fn test_default_v0_2_runner_includes_conf_and_security() {
    let mut ctx = setup_ctx(&[]).await;

    let mut runner = CheckRunner::new();
    runner.register_default_v0_2_checks();

    let results = runner.run_all(&mut ctx).await;
    let _ = ctx.session.shutdown().await;

    let has_conf = results.iter().any(|r| r.check_id.starts_with("CONF-"));
    let has_sec = results.iter().any(|r| r.check_id.starts_with("SEC-"));

    assert!(has_conf, "Default v0.2 runner must include CONF-* checks");
    assert!(has_sec, "Default v0.2 runner must include SEC-* checks");
}
