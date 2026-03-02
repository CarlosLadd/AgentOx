//! Minimal MCP test server built with the official Rust SDK (rmcp).
//!
//! Exposes 3 simple tools (add, echo, reverse) via stdio transport.
//! Designed as a real-world audit target for AgentOx.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_handler, tool_router, ServiceExt};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Tool parameter types — schemars generates JSON Schema automatically
// ---------------------------------------------------------------------------

/// Parameters for the `add` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddRequest {
    /// First number to add.
    pub a: f64,
    /// Second number to add.
    pub b: f64,
}

/// Parameters for the `echo` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EchoRequest {
    /// The message to echo back.
    pub message: String,
}

/// Parameters for the `reverse` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReverseRequest {
    /// The text to reverse.
    pub text: String,
}

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TestServer {
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl TestServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Add two numbers and return the sum.
    #[tool(description = "Add two numbers and return the sum")]
    async fn add(&self, Parameters(req): Parameters<AddRequest>) -> String {
        let sum = req.a + req.b;
        sum.to_string()
    }

    /// Echo the input message back.
    #[tool(description = "Echo the input message back")]
    async fn echo(&self, Parameters(req): Parameters<EchoRequest>) -> String {
        req.message
    }

    /// Reverse a string.
    #[tool(description = "Reverse a string")]
    async fn reverse(&self, Parameters(req): Parameters<ReverseRequest>) -> String {
        req.text.chars().rev().collect()
    }
}

#[tool_handler]
impl rmcp::ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mcp-test-server-rust".into(),
                version: "0.1.0".into(),
                ..Default::default()
            },
            instructions: Some(
                "A minimal MCP test server for AgentOx conformance auditing.".into(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = TestServer::new()
        .serve(rmcp::transport::io::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
