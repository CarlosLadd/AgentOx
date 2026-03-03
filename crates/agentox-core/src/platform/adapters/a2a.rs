use crate::client::session::McpSession;
use crate::error::{SessionError, TransportError};
use crate::platform::adapter::ProtocolAdapter;
use crate::platform::adapters::mcp::McpProtocolAdapter;
use crate::platform::types::{AdapterMetadata, AgentProtocol, AgentToolCallResult, AgentToolModel};
use crate::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::Implementation;

/// Basic A2A adapter using the current JSON-RPC request/response compatibility path.
///
/// This keeps transport and check execution stable while non-MCP protocol-specific
/// semantics continue to evolve in future releases.
pub struct A2aProtocolAdapter {
    inner: McpProtocolAdapter,
}

impl A2aProtocolAdapter {
    pub fn new(session: McpSession) -> Self {
        Self {
            inner: McpProtocolAdapter::new(session),
        }
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for A2aProtocolAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::A2a
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: "a2a-basic-compat".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    async fn initialize(
        &mut self,
    ) -> Result<crate::protocol::mcp_types::InitializeResult, SessionError> {
        self.inner.initialize().await
    }

    async fn list_tools(&mut self) -> Result<Vec<AgentToolModel>, SessionError> {
        self.inner.list_tools().await
    }

    async fn invoke_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<AgentToolCallResult, SessionError> {
        self.inner.invoke_tool(name, arguments).await
    }

    async fn send_probe(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        self.inner.send_probe(req).await
    }

    async fn send_raw_probe(&mut self, raw: &str) -> Result<Option<String>, TransportError> {
        self.inner.send_raw_probe(raw).await
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        self.inner.shutdown().await
    }

    fn next_id(&self) -> i64 {
        self.inner.next_id()
    }

    fn protocol_version(&self) -> Option<&str> {
        self.inner.protocol_version()
    }

    fn server_info(&self) -> Option<&Implementation> {
        self.inner.server_info()
    }
}
