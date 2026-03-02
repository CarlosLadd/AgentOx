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
```

### Expected Results

9/10 conformance checks pass. CONF-005 (malformed request handling) fails because the rmcp SDK disconnects on truncated JSON instead of returning a JSON-RPC parse error (`-32700`). This is a known limitation of the SDK, not of the server implementation.

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
```
