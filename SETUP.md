# AgentOx Setup Guide

Complete instructions for building, testing, and running AgentOx from source.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Clone and Build](#clone-and-build)
3. [Project Structure](#project-structure)
4. [Running Tests](#running-tests)
5. [Install Locally](#install-locally)
6. [CLI Reference](#cli-reference)
7. [Usage Examples](#usage-examples)
8. [Troubleshooting](#troubleshooting)

---

## Prerequisites

| Requirement  | Minimum Version | Check Command        |
|--------------|-----------------|----------------------|
| Rust toolchain | 1.93.1        | `rustc --version`    |
| Cargo        | (bundled with Rust) | `cargo --version` |
| Git          | any             | `git --version`      |

Install Rust via [rustup](https://rustup.rs) if you don't have it:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

No other system dependencies are required. AgentOx compiles to a single static binary.

---

## Clone and Build

### 1. Clone the repository

```sh
git clone https://github.com/CarlosLadd/AgentOx.git
cd AgentOx
```

### 2. Build the entire workspace (debug mode)

```sh
cargo build --workspace
```

This compiles three targets:

| Target | Binary | Description |
|--------|--------|-------------|
| `agentox-cli` | `target/debug/agentox` | The main CLI tool |
| `agentox-core` | *(library)* | Core audit engine |
| `agentox-mock-server` | `target/debug/mock-mcp-server` | Mock MCP server for testing |

**Verify the build succeeded:**

```sh
./target/debug/agentox --version
```

Expected output:

```
agentox 0.1.0
```

### 3. Build in release mode (optimized)

```sh
cargo build --release
```

The release binary is at `target/release/agentox`.

---

## Project Structure

```
AgentOx/
├── Cargo.toml                          # Workspace root
├── CLAUDE.md                           # Project conventions
├── README.md                           # Project overview
├── SETUP.md                            # This file
├── LICENSE-MIT
├── LICENSE-APACHE
├── crates/
│   ├── agentox-core/                   # Core library — all audit logic
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                  # Public API re-exports
│   │   │   ├── error.rs                # Error types (TransportError, SessionError)
│   │   │   ├── protocol/
│   │   │   │   ├── jsonrpc.rs          # JSON-RPC 2.0 message types
│   │   │   │   └── mcp_types.rs        # MCP protocol types
│   │   │   ├── client/
│   │   │   │   ├── transport.rs        # Transport trait
│   │   │   │   ├── stdio.rs            # Stdio subprocess transport
│   │   │   │   └── session.rs          # MCP session + handshake
│   │   │   ├── checks/
│   │   │   │   ├── types.rs            # CheckResult, Severity, CheckCategory
│   │   │   │   ├── runner.rs           # Check trait + CheckRunner
│   │   │   │   └── conformance/        # 10 conformance check modules
│   │   │   │       ├── init_capabilities.rs      # CONF-001
│   │   │   │       ├── jsonrpc_structure.rs      # CONF-002
│   │   │   │       ├── tools_list_valid.rs       # CONF-003
│   │   │   │       ├── tool_input_schema.rs      # CONF-004
│   │   │   │       ├── malformed_request.rs      # CONF-005
│   │   │   │       ├── unknown_method.rs         # CONF-006
│   │   │   │       ├── error_codes.rs            # CONF-007
│   │   │   │       ├── capability_negotiation.rs # CONF-008
│   │   │   │       ├── protocol_version.rs       # CONF-009
│   │   │   │       └── initialized_order.rs      # CONF-010
│   │   │   └── report/
│   │   │       ├── types.rs            # AuditReport, AuditSummary
│   │   │       ├── text.rs             # Colored terminal renderer
│   │   │       └── json.rs             # JSON renderer
│   │   └── tests/
│   │       └── integration_test.rs     # 5 integration tests
│   └── agentox-cli/                    # Thin CLI binary wrapper
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                 # Clap argument parsing + audit pipeline
└── tests/
    └── mock_server/                    # Mock MCP server (separate binary)
        ├── Cargo.toml
        └── main.rs
```

---

## Running Tests

### Run the full test suite

```sh
cargo test --workspace
```

This runs all integration tests against the mock MCP server. Expected output:

```
running 5 tests
test test_unknown_method_returns_minus_32601 ... ok
test test_conformant_server_passes_all_checks ... ok
test test_json_report_is_valid_json_with_required_fields ... ok
test test_report_summary_counts_are_consistent ... ok
test test_server_with_no_tools_still_passes_init_check ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Run a specific test

```sh
cargo test -p agentox-core --test integration_test test_conformant_server_passes_all_checks
```

### Run tests with verbose output

```sh
cargo test --workspace -- --nocapture
```

### What the integration tests cover

| Test | What it verifies |
|------|-----------------|
| `test_conformant_server_passes_all_checks` | All 10 conformance checks pass against a well-behaved server |
| `test_report_summary_counts_are_consistent` | `passed + failed == total_checks` in the report summary |
| `test_json_report_is_valid_json_with_required_fields` | JSON output is parseable and contains `summary`, `results`, `agentox_version` |
| `test_server_with_no_tools_still_passes_init_check` | CONF-001 passes even when the server declares no tools capability |
| `test_unknown_method_returns_minus_32601` | Server returns the correct JSON-RPC error code for unknown methods |

### Code quality checks

```sh
# Lint with clippy (should produce zero warnings)
cargo clippy --workspace --all-targets

# Verify formatting
cargo fmt --all -- --check

# Auto-fix formatting if needed
cargo fmt --all
```

---

## Install Locally

### Option A: Install from the workspace

```sh
cargo install --path crates/agentox-cli
```

This places the `agentox` binary in `~/.cargo/bin/` (make sure this is in your `$PATH`).

**Verify:**

```sh
agentox --version
```

### Option B: Use without installing

Run directly from the build output:

```sh
# Debug build
./target/debug/agentox audit --stdio "your-server-command"

# Release build (faster)
./target/release/agentox audit --stdio "your-server-command"
```

---

## CLI Reference

### Global options

```
agentox [OPTIONS] <COMMAND>

Options:
  -v, --verbose    Enable verbose/debug logging (output goes to stderr)
  -h, --help       Print help
  -V, --version    Print version
```

### `agentox audit`

Run a full audit against an MCP server.

```
agentox audit [OPTIONS]

Options:
  --stdio <COMMAND>      Server command for stdio transport
  --target <URL>         Server HTTP/SSE endpoint URL (reserved, not implemented yet)
  --format <FORMAT>      Output format: text (default) or json [default: text]
  --only <CATEGORY>      Run only specific check categories: conformance, security, behavioral
  --timeout <SECONDS>    Per-check timeout in seconds [default: 30]
  --no-color             Disable colored output
```

In v0.2, `--stdio` is required. `--target` is reserved for a future HTTP/SSE transport.

### Exit codes

| Code | Meaning |
|------|---------|
| `0`  | All checks passed |
| `1`  | One or more checks failed (findings detected) |

---

## Usage Examples

### Basic audit of an MCP server

Audit a server that runs via stdio (the most common pattern):

```sh
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp"
```

Sample output:

```
AgentOx Audit Report
Target: npx -y @modelcontextprotocol/server-filesystem /tmp
Server: filesystem v0.6.2
Protocol: 2025-11-25

[PASS] CONF-001 Initialize returns valid capabilities
[PASS] CONF-002 JSON-RPC 2.0 message structure
[PASS] CONF-003 tools/list returns valid tools
[PASS] CONF-004 Tool inputSchema is valid JSON Schema
[PASS] CONF-005 Malformed request handling
[PASS] CONF-006 Unknown method handling
[PASS] CONF-007 Error code correctness
[PASS] CONF-008 Capability negotiation
[PASS] CONF-009 Protocol version validation
[PASS] CONF-010 Initialized notification handling

Summary
  Total: 10, Passed: 10, Failed: 0
  Duration: 342ms
```

### Audit with JSON output

Structured JSON is ideal for CI pipelines and programmatic consumption:

```sh
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" --format json
```

Pipe through `jq` for pretty-printing:

```sh
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" \
  --format json 2>/dev/null | jq .
```

Sample JSON structure:

```json
{
  "agentox_version": "0.1.0",
  "timestamp": "2026-03-01T23:56:51.627703+00:00",
  "target": "npx -y @modelcontextprotocol/server-filesystem /tmp",
  "server_info": {
    "name": "filesystem",
    "version": "0.6.2"
  },
  "protocol_version": "2025-11-25",
  "results": [
    {
      "check_id": "CONF-001",
      "name": "Initialize returns valid capabilities",
      "category": "conformance",
      "severity": "PASS",
      "passed": true,
      "description": "Validates that the initialize response contains ...",
      "message": "Check passed",
      "duration_ms": 3
    }
  ],
  "summary": {
    "total_checks": 10,
    "passed": 10,
    "failed": 0,
    "duration_ms": 342
  }
}
```

### Audit the built-in mock server

Useful for verifying your build is working correctly:

```sh
# Build the workspace first
cargo build --workspace

# Audit the mock server
agentox audit --stdio "./target/debug/mock-mcp-server"
```

### Verbose mode for debugging

Enable debug-level logging (logs go to stderr, report goes to stdout):

```sh
agentox -v audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp"
```

### Disable colored output

For environments without ANSI support, or when piping to a file:

```sh
agentox audit --stdio "your-server-command" --no-color
```

You can also set the `NO_COLOR` environment variable:

```sh
NO_COLOR=1 agentox audit --stdio "your-server-command"
```

### Save a report to a file

```sh
# Text report
agentox audit --stdio "your-server-command" --no-color > report.txt 2>/dev/null

# JSON report
agentox audit --stdio "your-server-command" --format json > report.json 2>/dev/null
```

Note: `2>/dev/null` suppresses the progress/status messages that go to stderr, keeping only the clean report in the file.

### Use in a CI pipeline

AgentOx returns exit code `1` when any check fails, making it a natural CI gate:

```sh
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" \
  --format json --no-color || {
    echo "MCP server failed audit — blocking deploy"
    exit 1
}
```

GitHub Actions example:

```yaml
jobs:
  mcp-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action/setup@v1

      - name: Install AgentOx
        run: cargo install agentox-cli

      - name: Audit MCP server
        run: |
          agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" \
            --format json --no-color | tee audit-report.json

      - name: Upload audit report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: agentox-report
          path: audit-report.json
```

### Audit a Python-based MCP server

```sh
agentox audit --stdio "python -m my_mcp_server"
```

### Audit a server with arguments containing spaces

Wrap the entire command in quotes:

```sh
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /path/to/my directory"
```

### Extract only failed checks from JSON output

```sh
agentox audit --stdio "your-server-command" --format json 2>/dev/null \
  | jq '.results[] | select(.passed == false)'
```

### Count failures programmatically

```sh
FAILURES=$(agentox audit --stdio "your-server-command" --format json 2>/dev/null \
  | jq '.summary.failed')
echo "Failed checks: $FAILURES"
```

---

## Troubleshooting

### `cargo build` fails with "edition 2021 is not supported"

Your Rust toolchain is too old. Update with:

```sh
rustup update stable
```

AgentOx requires Rust 1.93.1 or later.

### Integration tests fail with "failed to spawn mock server"

The mock server binary must be built before running tests. Run:

```sh
cargo build --workspace
cargo test --workspace
```

### "MCP handshake failed" when auditing a server

The target server either crashed during startup or does not speak MCP. Check that:

1. The server command runs successfully on its own (e.g., `npx -y @modelcontextprotocol/server-filesystem /tmp`)
2. The server uses stdio (stdin/stdout) for communication
3. The server implements the MCP `initialize` handshake

Use verbose mode to see the raw JSON-RPC exchange:

```sh
agentox -v audit --stdio "your-server-command"
```

### "HTTP/SSE transport is not yet implemented"

The `--target` flag for HTTP/SSE servers is still pending. For now, use `--stdio` with a command that spawns the server process.

### No colored output in my terminal

Some terminals or CI environments don't support ANSI colors. If colors look garbled, use `--no-color`. If you expected colors but don't see them, make sure your terminal emulator supports ANSI escape codes.

---

## Conformance Checks Reference

All 10 conformance checks included in v0.2.0:

| ID | Name | What it validates | Severity on Fail |
|----|------|------------------|-----------------|
| CONF-001 | Initialize Capabilities | `initialize` returns valid `protocolVersion`, `capabilities`, `serverInfo` | CRITICAL |
| CONF-002 | JSON-RPC Structure | All responses have `jsonrpc: "2.0"`, matching `id`, one of `result`/`error` | HIGH |
| CONF-003 | Tools List Valid | `tools/list` returns tools with `name` + `inputSchema`, no duplicates | HIGH |
| CONF-004 | Tool Input Schema | Each tool's `inputSchema` is valid JSON Schema with `type: "object"` | HIGH |
| CONF-005 | Malformed Request Handling | Server returns proper errors (not crash) for broken JSON, missing fields | CRITICAL |
| CONF-006 | Unknown Method Handling | Server returns `-32601` for non-existent methods | HIGH |
| CONF-007 | Error Code Correctness | Error responses use standard JSON-RPC codes (-32700, -32600, -32601, etc.) | MEDIUM |
| CONF-008 | Capability Negotiation | Declared capabilities match actual method support | MEDIUM |
| CONF-009 | Protocol Version | Server handles version negotiation correctly | HIGH |
| CONF-010 | Initialized Notification | Server handles `notifications/initialized` lifecycle correctly | LOW |

## Security Checks Reference

Initial security checks included in v0.2.0:

| ID | Name | What it validates | Severity on Fail |
|----|------|------------------|-----------------|
| SEC-001 | Prompt-injection Echo Safety | Injection-like tool input is handled safely or rejected | HIGH |
| SEC-002 | Tool Parameter Boundary Validation | `tools/call` malformed parameters are rejected with JSON-RPC errors | HIGH |
| SEC-003 | Error Leakage Detection | Error bodies do not leak sensitive internals (paths, traces, secret-like tokens) | MEDIUM |
| SEC-004 | Resource-exhaustion Guardrail | Server stays responsive under bounded burst + large-input probes | MEDIUM |
