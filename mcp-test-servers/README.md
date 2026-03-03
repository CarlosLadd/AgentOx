# MCP Test Servers

Minimal MCP servers designed as audit targets for [AgentOx](https://github.com/CarlosLadd/AgentOx).

## Rust SDK Server (`rust-sdk-server`)

Built with the official Rust MCP SDK ([rmcp](https://crates.io/crates/rmcp) v0.17).

### Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `add` | `a: f64, b: f64` | Add two numbers and return the sum |
| `echo` | `message: String` | Echo the input message back |
| `reverse` | `text: String` | Reverse a string |

### Build

```sh
cargo build -p mcp-test-server-rust
```

### Run

```sh
./target/debug/mcp-test-server-rust
```

The server communicates via stdio (stdin/stdout JSON-RPC).

### Audit with AgentOx

```sh
# Text report
agentox audit --stdio "./target/debug/mcp-test-server-rust"

# JSON report
agentox audit --stdio "./target/debug/mcp-test-server-rust" --format json

# Security-only checks
agentox audit --stdio "./target/debug/mcp-test-server-rust" --only security
```

### Expected Results

For AgentOx `v1.0.0`, expected baseline is **16/17**:
- `CONF-001..004`, `CONF-006..010` pass
- `SEC-001..004` pass
- `BHV-001..003` pass
- `CONF-005` fails

`CONF-005` (malformed request handling) fails because the rmcp SDK disconnects on truncated JSON instead of returning a JSON-RPC parse error (`-32700`). This is a known SDK limitation, not a server business-logic bug.

```
  [PASS] CONF-001 Initialize returns valid capabilities
  [PASS] CONF-002 JSON-RPC 2.0 message structure
  [PASS] CONF-003 tools/list returns valid tools
  [PASS] CONF-004 Tool inputSchema is valid JSON Schema
  [FAIL] CONF-005 Malformed request handling
  [PASS] CONF-006 Unknown method handling
  [PASS] CONF-007 Error code correctness
  [PASS] CONF-008 Capability negotiation
  [PASS] CONF-009 Protocol version validation
  [PASS] CONF-010 Initialized notification handling
  [PASS] SEC-001 Prompt-injection echo safety
  [PASS] SEC-002 Tool parameter boundary validation
  [PASS] SEC-003 Error leakage detection
  [PASS] SEC-004 Resource-exhaustion guardrail
  [PASS] BHV-001 Idempotency baseline
  [PASS] BHV-002 Schema-output alignment
  [PASS] BHV-003 Deterministic error semantics
```

### Expected Failure Policy

`CONF-005` is an expected failure for this test target. Treat it as non-blocking for routine test-server maintenance unless the behavior changes upstream in rmcp or AgentOx intentionally tightens/changes policy.
