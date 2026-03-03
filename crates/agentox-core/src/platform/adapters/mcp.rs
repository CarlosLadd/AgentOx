use crate::client::session::McpSession;
use crate::error::{SessionError, TransportError};
use crate::platform::adapter::ProtocolAdapter;
use crate::platform::types::{AdapterMetadata, AgentProtocol, AgentToolCallResult, AgentToolModel};
use crate::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::Implementation;

pub struct McpProtocolAdapter {
    session: McpSession,
}

impl McpProtocolAdapter {
    pub fn new(session: McpSession) -> Self {
        Self { session }
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for McpProtocolAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::Mcp
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: "mcp-compat".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    async fn initialize(
        &mut self,
    ) -> Result<crate::protocol::mcp_types::InitializeResult, SessionError> {
        self.session.initialize().await
    }

    async fn list_tools(&mut self) -> Result<Vec<AgentToolModel>, SessionError> {
        let tools = self.session.list_tools().await?;
        Ok(tools.into_iter().map(Into::into).collect())
    }

    async fn invoke_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<AgentToolCallResult, SessionError> {
        let result = self.session.call_tool(name, arguments).await?;
        Ok(result.into())
    }

    async fn send_probe(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        self.session.send_request(req).await
    }

    async fn send_raw_probe(&mut self, raw: &str) -> Result<Option<String>, TransportError> {
        self.session.send_raw(raw).await
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        self.session.shutdown().await
    }

    fn next_id(&self) -> i64 {
        self.session.next_id()
    }

    fn protocol_version(&self) -> Option<&str> {
        self.session.protocol_version()
    }

    fn server_info(&self) -> Option<&Implementation> {
        self.session.server_info()
    }
}
