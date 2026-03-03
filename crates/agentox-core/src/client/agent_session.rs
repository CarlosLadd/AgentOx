//! Protocol-adapter session wrapper for multi-protocol audits.

use crate::error::{SessionError, TransportError};
use crate::platform::adapter::ProtocolAdapter;
use crate::platform::types::{AdapterMetadata, AgentProtocol};
use crate::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::{CallToolResult, Tool};

pub struct AgentSession {
    adapter: Box<dyn ProtocolAdapter>,
}

impl AgentSession {
    pub fn new(adapter: Box<dyn ProtocolAdapter>) -> Self {
        Self { adapter }
    }

    pub async fn initialize(
        &mut self,
    ) -> Result<crate::protocol::mcp_types::InitializeResult, SessionError> {
        self.adapter.initialize().await
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, SessionError> {
        let tools = self.adapter.list_tools().await?;
        Ok(tools.into_iter().map(Into::into).collect())
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, SessionError> {
        let result = self.adapter.invoke_tool(name, arguments).await?;
        Ok(result.into())
    }

    pub async fn send_raw(&mut self, raw: &str) -> Result<Option<String>, TransportError> {
        self.adapter.send_raw_probe(raw).await
    }

    pub async fn send_request(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        self.adapter.send_probe(req).await
    }

    pub async fn shutdown(&mut self) -> Result<(), TransportError> {
        self.adapter.shutdown().await
    }

    pub fn next_id(&self) -> i64 {
        self.adapter.next_id()
    }

    pub fn protocol_version(&self) -> Option<&str> {
        self.adapter.protocol_version()
    }

    pub fn server_info(&self) -> Option<&crate::protocol::mcp_types::Implementation> {
        self.adapter.server_info()
    }

    pub fn protocol(&self) -> AgentProtocol {
        self.adapter.protocol()
    }

    pub fn adapter_metadata(&self) -> AdapterMetadata {
        self.adapter.metadata()
    }
}
