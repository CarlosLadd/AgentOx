//! Minimal mock MCP server for integration testing.
//!
//! Reads from stdin, writes to stdout. Controlled via env vars:
//!   MOCK_CRASH_ON_INIT=1  — exits immediately on initialize
//!   MOCK_BAD_JSONRPC=1    — responds with jsonrpc: "1.0" instead of "2.0"
//!   MOCK_NO_TOOLS=1       — declares no tools capability
//!   MOCK_EMPTY_TOOLS=1    — returns an empty tools list

use std::io::{BufRead, Write};

fn tool_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "A test message"
            }
        },
        "required": []
    })
}

fn handle_message(
    line: &str,
    crash_on_init: bool,
    bad_jsonrpc: bool,
    no_tools: bool,
    empty_tools: bool,
) -> Option<String> {
    let msg: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            // Return parse error
            return Some(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                })
                .to_string(),
            );
        }
    };

    let jsonrpc_version = if bad_jsonrpc { "1.0" } else { "2.0" };
    let id = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // Notifications have no id, no response needed
    msg.get("id")?;

    // Validate jsonrpc field (a conformant server should check this)
    if !bad_jsonrpc {
        match msg.get("jsonrpc").and_then(|v| v.as_str()) {
            Some("2.0") => {} // valid
            _ => {
                return Some(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32600,
                            "message": "Invalid Request: missing or wrong jsonrpc version"
                        }
                    })
                    .to_string(),
                );
            }
        }
    }

    let response = match method {
        "initialize" => {
            if crash_on_init {
                std::process::exit(1);
            }

            let caps = if no_tools {
                serde_json::json!({})
            } else {
                serde_json::json!({ "tools": { "listChanged": false } })
            };

            serde_json::json!({
                "jsonrpc": jsonrpc_version,
                "id": id,
                "result": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": caps,
                    "serverInfo": {
                        "name": "mock-mcp-server",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "tools/list" => {
            let tools = if empty_tools || no_tools {
                serde_json::json!([])
            } else {
                serde_json::json!([
                    {
                        "name": "echo",
                        "description": "Echoes the input message",
                        "inputSchema": tool_schema()
                    },
                    {
                        "name": "greet",
                        "description": "Returns a greeting",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" }
                            }
                        }
                    }
                ])
            };

            serde_json::json!({
                "jsonrpc": jsonrpc_version,
                "id": id,
                "result": { "tools": tools }
            })
        }
        "tools/call" => {
            let tool_name = msg
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");

            if tool_name == "echo" || tool_name == "greet" {
                serde_json::json!({
                    "jsonrpc": jsonrpc_version,
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": "mock response" }],
                        "isError": false
                    }
                })
            } else {
                serde_json::json!({
                    "jsonrpc": jsonrpc_version,
                    "id": id,
                    "error": { "code": -32602, "message": "Unknown tool" }
                })
            }
        }
        _ => {
            serde_json::json!({
                "jsonrpc": jsonrpc_version,
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            })
        }
    };

    Some(response.to_string())
}

fn main() {
    let crash_on_init = std::env::var("MOCK_CRASH_ON_INIT").is_ok();
    let bad_jsonrpc = std::env::var("MOCK_BAD_JSONRPC").is_ok();
    let no_tools = std::env::var("MOCK_NO_TOOLS").is_ok();
    let empty_tools = std::env::var("MOCK_EMPTY_TOOLS").is_ok();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        if let Some(response) =
            handle_message(&line, crash_on_init, bad_jsonrpc, no_tools, empty_tools)
        {
            writeln!(out, "{response}").unwrap();
            out.flush().unwrap();
        }
    }
}
