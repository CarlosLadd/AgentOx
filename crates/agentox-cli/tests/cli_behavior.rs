use std::process::Command;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

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

fn read_http_request(stream: &mut TcpStream) -> Option<String> {
    let mut buf = vec![0_u8; 8192];
    let mut read = 0usize;
    loop {
        let n = stream.read(&mut buf[read..]).ok()?;
        if n == 0 {
            break;
        }
        read += n;
        if read >= 4 && buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if read >= buf.len() {
            buf.resize(buf.len() * 2, 0);
        }
    }
    Some(String::from_utf8_lossy(&buf[..read]).to_string())
}

fn extract_body(raw: &str) -> String {
    raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string()
}

fn write_http_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn start_http_mock_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test http server");
    let addr = listener.local_addr().expect("server local_addr");
    thread::spawn(move || {
        for _ in 0..64 {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(_) => break,
            };
            let Some(raw_req) = read_http_request(&mut stream) else {
                continue;
            };
            let body = extract_body(&raw_req);
            let parsed = serde_json::from_str::<serde_json::Value>(&body).unwrap_or_default();
            let id = parsed.get("id").cloned().unwrap_or(serde_json::Value::Null);
            let method = parsed
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if parsed.get("id").is_none() {
                write_http_response(&mut stream, "200 OK", "application/json", "");
                continue;
            }

            let resp = match method {
                "initialize" => serde_json::json!({
                    "jsonrpc":"2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": {"listChanged": false} },
                        "serverInfo": { "name": "http-mock", "version": "0.1.0" }
                    }
                }),
                "tools/list" => serde_json::json!({
                    "jsonrpc":"2.0",
                    "id": id,
                    "result": { "tools": [] }
                }),
                "tools/call" => serde_json::json!({
                    "jsonrpc":"2.0",
                    "id": id,
                    "error": { "code": -32602, "message": "Invalid params" }
                }),
                _ => serde_json::json!({
                    "jsonrpc":"2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "Method not found" }
                }),
            };
            write_http_response(&mut stream, "200 OK", "application/json", &resp.to_string());
        }
    });

    format!("http://{}", addr)
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
fn test_only_behavioral_executes_behavioral_checks() {
    let mock_bin = ensure_mock_server_binary();
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args([
            "audit",
            "--stdio",
            &mock_bin,
            "--only",
            "behavioral",
            "--format",
            "text",
            "--no-color",
        ])
        .output()
        .expect("failed to run agentox");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BHV-001"),
        "expected behavioral checks in output"
    );
    assert!(
        !stdout.contains("CONF-001") && !stdout.contains("SEC-001"),
        "did not expect non-behavioral checks in output"
    );
}

#[test]
fn test_target_flag_performs_audit() {
    let endpoint = start_http_mock_server();
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args([
            "audit",
            "--target",
            &endpoint,
            "--only",
            "conformance",
            "--format",
            "text",
            "--no-color",
        ])
        .output()
        .expect("failed to run agentox");
    assert!(
        output.status.code().is_some(),
        "--target audit should produce an exit code"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CONF-001"),
        "expected conformance output via --target"
    );
}

#[test]
fn test_stdio_and_target_together_fail() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentox"))
        .args([
            "audit",
            "--stdio",
            "/bin/echo",
            "--target",
            "http://127.0.0.1:1",
        ])
        .output()
        .expect("failed to run agentox");
    assert!(
        !output.status.success(),
        "dual target selection should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Use either --stdio or --target, but not both."),
        "expected clear dual-target error, got: {stderr}"
    );
}
