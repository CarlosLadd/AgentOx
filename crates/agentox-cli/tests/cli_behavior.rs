use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest)
        .join("../..")
        .canonicalize()
        .expect("cannot resolve workspace root")
}

fn ensure_mock_server_binary() -> String {
    let status = Command::new("cargo")
        .args(["build", "-p", "agentox-mock-server"])
        .current_dir(workspace_root())
        .status()
        .expect("failed to invoke cargo build for mock server");
    assert!(status.success(), "building mock server failed");

    workspace_root()
        .join("target/debug/mock-mcp-server")
        .to_string_lossy()
        .to_string()
}

#[test]
fn test_only_security_executes_security_checks() {
    let mock_bin = ensure_mock_server_binary();
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args([
            "audit",
            "--stdio",
            &mock_bin,
            "--only",
            "security",
            "--format",
            "text",
            "--no-color",
        ])
        .output()
        .expect("failed to run agentox");
    assert!(
        output.status.code().is_some(),
        "security-only run should produce a process exit code"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SEC-001"),
        "expected security checks in output"
    );
    assert!(
        !stdout.contains("CONF-001"),
        "did not expect conformance checks"
    );
}

#[test]
fn test_only_conformance_executes_conformance_checks() {
    let mock_bin = ensure_mock_server_binary();
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args([
            "audit",
            "--stdio",
            &mock_bin,
            "--only",
            "conformance",
            "--format",
            "text",
            "--no-color",
        ])
        .output()
        .expect("failed to run agentox");
    assert!(
        output.status.success(),
        "conformance-only run should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CONF-001"),
        "expected conformance checks in output"
    );
    assert!(
        !stdout.contains("SEC-001"),
        "did not expect security checks"
    );
}

#[test]
fn test_only_behavioral_fails_fast() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args(["audit", "--stdio", "/bin/echo", "--only", "behavioral"])
        .output()
        .expect("failed to run agentox");
    assert!(!output.status.success(), "behavioral mode should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Behavioral checks are not implemented yet."),
        "expected clear behavioral-not-implemented error, got: {stderr}"
    );
}

#[test]
fn test_target_flag_still_reports_not_implemented() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args(["audit", "--target", "http://localhost:8080"])
        .output()
        .expect("failed to run agentox");
    assert!(!output.status.success(), "--target should fail for now");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("HTTP/SSE transport is not yet implemented"),
        "expected explicit target-not-implemented error, got: {stderr}"
    );
}
