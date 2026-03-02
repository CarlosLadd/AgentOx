# AgentOx

**MCP Security & Conformance Auditor — Forged in Rust**

[![Crates.io](https://img.shields.io/crates/v/agentox.svg)](https://crates.io/crates/agentox)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

AgentOx is the open-source CLI tool for auditing [MCP (Model Context Protocol)](https://modelcontextprotocol.io) servers — checking protocol conformance, security surface, and behavioral contracts before any server ships to production.

> *Think `trivy` for containers, but for AI agent infrastructure.*

---

## Installation

```sh
cargo install agentox-cli
```

Or build from source:

```sh
git clone https://github.com/CarlosLadd/AgentOx.git
cd AgentOx
cargo install --path crates/agentox-cli
```

---

## Quick Start

```sh
# Audit any MCP server via stdio
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp"

# Output structured JSON for CI pipelines
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" --format json

# Audit an HTTP/SSE server (coming in v0.2)
agentox audit --target http://localhost:8080
```

---

## Example Output

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

---

## Conformance Checks (v0.1.0)

| ID | Check | Severity on Fail |
|----|-------|-----------------|
| CONF-001 | `initialize` returns valid `protocolVersion`, `capabilities`, `serverInfo` | CRITICAL |
| CONF-002 | All responses are valid JSON-RPC 2.0 | HIGH |
| CONF-003 | `tools/list` returns valid tool definitions, no duplicates | HIGH |
| CONF-004 | Tool `inputSchema` is valid JSON Schema with `type: "object"` | HIGH |
| CONF-005 | Malformed requests return errors, server does not crash | CRITICAL |
| CONF-006 | Unknown methods return `-32601` | HIGH |
| CONF-007 | Error codes use standard JSON-RPC values | MEDIUM |
| CONF-008 | Declared capabilities match supported methods | MEDIUM |
| CONF-009 | Protocol version negotiation is handled correctly | HIGH |
| CONF-010 | Initialization lifecycle is handled correctly | LOW |

---

## CI/CD Integration

AgentOx exits with code `1` when findings are detected and `0` when all checks pass — making it a natural fit for CI pipelines.

### GitHub Actions

```yaml
- name: Audit MCP server
  run: |
    cargo install agentox-cli
    agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" \
      --format json --no-color | tee audit-report.json

- name: Upload audit report
  uses: actions/upload-artifact@v4
  with:
    name: agentox-report
    path: audit-report.json
```

### Shell Script

```sh
agentox audit --stdio "npx my-mcp-server" || {
  echo "MCP server failed audit — blocking deploy"
  exit 1
}
```

---

## Roadmap

| Version | Focus | Status |
|---------|-------|--------|
| **v0.1** | Protocol Conformance (10 checks, stdio transport) | ✅ Done |
| **v0.2** | Security Surface Analysis (prompt injection, SSRF, error leakage) | 🔨 Planned |
| **v0.3** | CI/CD Integration (GitHub Action, Docker image, exit codes) | 📋 Planned |
| **v0.4** | Behavioral Contracts (idempotency, schema-output alignment) | 📋 Planned |
| **v1.0** | Stable API, HTTP/SSE transport, HTML reports | 📋 Planned |

---

## Architecture

AgentOx is a Cargo workspace with a clean library/CLI separation:

```
crates/
├── agentox-core/   # Core audit engine (embeddable library)
│   ├── client/     # MCP transport + session
│   ├── checks/     # Audit checks (conformance, security, behavioral)
│   ├── protocol/   # JSON-RPC 2.0 + MCP 2025-11-25 types
│   └── report/     # Text + JSON report renderers
└── agentox-cli/    # Thin CLI wrapper (main.rs + clap)
```

**Key design principle:** AgentOx uses its own JSON-RPC types (not an external MCP SDK) so it can send intentionally malformed messages to test server robustness — something a well-behaved SDK would reject client-side.

---

## Why Rust?

- **Single binary** — `cargo install agentox-cli`, zero runtime, runs in any CI pipeline
- **Memory safety** — auditing tools built in unsafe languages create irony
- **Speed** — fuzz-test hundreds of variants per second; Python tools are too slow
- **'Ox' brand** — Agent + Oxidize. The name and language are inseparable

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

To add a new check:
1. Create `crates/agentox-core/src/checks/conformance/my_check.rs`
2. Implement the `Check` trait
3. Register it in `CheckRunner::register_conformance_checks()`
4. Add a unit test in the same file

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
