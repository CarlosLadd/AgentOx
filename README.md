# AgentOx

**Agentic Tool Security Platform (MCP + A2A + OpenAI tool_use) — Forged in Rust**

[![Crates.io](https://img.shields.io/crates/v/agentox-cli.svg)](https://crates.io/crates/agentox-cli)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

AgentOx is the open-source CLI tool for auditing agent tool servers across [MCP (Model Context Protocol)](https://modelcontextprotocol.io), A2A, and OpenAI tool_use compatibility paths — checking conformance, security surface, and behavioral contracts before any server ships to production.

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

# Run only security checks
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" --only security

# Audit an HTTP/SSE endpoint
agentox audit --target "http://localhost:8080"

# Select protocol adapter explicitly
agentox audit --target "http://localhost:8080" --protocol a2a

# Enforce policy-as-code and baseline regression guard
agentox audit --stdio "npx -y @modelcontextprotocol/server-filesystem /tmp" \
  --policy agentox-policy.yaml --baseline previous-report.json
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
[PASS] SEC-001 Prompt-injection echo safety
[PASS] SEC-002 Tool parameter boundary validation
[PASS] SEC-003 Error leakage detection
[PASS] SEC-004 Resource-exhaustion guardrail

Summary
  Total: 17, Passed: 17, Failed: 0
  Duration: 342ms
```

---

## Conformance Checks (v1.0.0)

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

## Security Checks (v1.0.0)

| ID | Check | Severity on Fail |
|----|-------|-----------------|
| SEC-001 | Prompt-injection style tool input is handled safely | HIGH |
| SEC-002 | `tools/call` parameter boundary validation | HIGH |
| SEC-003 | Error messages do not leak sensitive internals | MEDIUM |
| SEC-004 | Bounded burst/large-input resilience | MEDIUM |

## Behavioral Checks (v1.0.0)

| ID | Check | Severity on Fail |
|----|-------|-----------------|
| BHV-001 | tools/list idempotency baseline | MEDIUM |
| BHV-002 | Declared outputSchema vs actual tools/call output | HIGH |
| BHV-003 | Deterministic malformed-request error semantics | MEDIUM |

---

## CI/CD Integration

AgentOx exits with code `1` when findings are detected and `0` when all checks pass — making it a natural fit for CI pipelines.

## Stable Contracts (v1.0.0)

The following interfaces are semver-stable starting in v1.0.0:
- CLI command and flag behavior (`agentox audit`, `--stdio|--target`, `--format`, `--only`, `--timeout`, exit semantics)
- JSON report contract emitted by `--format json` (includes `schema_version: "1.0"`)

`agentox-core` internal Rust APIs are not yet semver-frozen.
HTML report generation is intentionally out of scope for v1.0.0.

## Vendor-Neutral Direction

AgentOx is positioned as a protocol-neutral security layer for agentic tool systems:
- Shared security and behavioral checks across protocol adapters
- Policy-as-code decisions for CI/CD (`--policy`)
- Portable evidence signatures in JSON output for downstream verification

### GitHub Actions

```yaml
- name: Audit tool server
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
  echo "Tool server failed audit — blocking deploy"
  exit 1
}
```

---

## Roadmap

| Version | Focus | Status |
|---------|-------|--------|
| **v0.1** | Protocol Conformance (10 checks, stdio transport) | ✅ Done |
| **v0.2** | Security Surface Analysis (initial `SEC-*` suite, stdio transport) | ✅ Done |
| **v0.3** | HTTP/SSE transport + CI/CD hardening | ✅ Done |
| **v0.4** | Behavioral Contracts + HTTP/SSE transport | ✅ Done |
| **v1.0** | Stable CLI/JSON API + HTTP/SSE transport GA (basic) | ✅ Done |
| **v1.0.1** | Reliability patch (transport/check hardening, diagnostics) | 📋 Planned |
| **v1.0.2** | DX patch (`--quiet`, `--output`, perf/documentation) | 📋 Planned |
| **v1.1** | Platform core extraction (protocol adapter layer + canonical model) | 📋 Planned |
| **v1.2** | Policy/evidence moat (`--policy`, baseline regression, signed evidence) | 📋 Planned |
| **v1.3** | A2A adapter GA + protocol capability matrix | 📋 Planned |
| **v1.4** | OpenAI tool_use adapter GA + cross-protocol parity | 📋 Planned |
| **v2.0** | Reserved for justified breaking changes only | ⏳ Criteria-based |

Detailed release-train plan: [ROADMAP.md](ROADMAP.md)

---

## Architecture

AgentOx is a Cargo workspace with a clean library/CLI separation:

```
crates/
├── agentox-core/   # Core audit engine (embeddable library)
│   ├── client/     # Transports + adapter-backed sessions
│   ├── checks/     # Audit checks (conformance, security, behavioral)
│   ├── platform/   # Protocol adapters + canonical tool model
│   ├── policy/     # Policy-as-code + baseline regression evaluation
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
