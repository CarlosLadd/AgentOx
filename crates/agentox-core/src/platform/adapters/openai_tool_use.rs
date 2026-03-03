use crate::client::session::McpSession;
use crate::error::{SessionError, TransportError};
use crate::platform::adapter::ProtocolAdapter;
use crate::platform::adapters::mcp::McpProtocolAdapter;
use crate::platform::types::{AdapterMetadata, AgentProtocol, AgentToolCallResult, AgentToolModel};
use crate::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::Implementation;

/// Basic OpenAI tool_use adapter using the current compatibility bridge.
///
/// For v1.x this adapter intentionally keeps one-request/one-response semantics.
pub struct OpenAiToolUseAdapter {
    inner: McpProtocolAdapter,
}

impl OpenAiToolUseAdapter {
    pub fn new(session: McpSession) -> Self {
        Self {
            inner: McpProtocolAdapter::new(session),
        }
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for OpenAiToolUseAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::OpenAiToolUse
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: "openai-tool-use-basic-compat".to_string(),
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
