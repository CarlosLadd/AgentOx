use crate::error::{SessionError, TransportError};
use crate::platform::types::{
    AdapterMetadata, AgentErrorClass, AgentErrorEnvelope, AgentProtocol, AgentToolCallResult,
    AgentToolModel,
};
use crate::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::{Implementation, InitializeResult};

#[async_trait::async_trait]
pub trait ProtocolAdapter: Send + Sync {
    fn protocol(&self) -> AgentProtocol;

    fn metadata(&self) -> AdapterMetadata;

    async fn initialize(&mut self) -> Result<InitializeResult, SessionError>;

    async fn list_tools(&mut self) -> Result<Vec<AgentToolModel>, SessionError>;

    async fn invoke_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<AgentToolCallResult, SessionError>;

    async fn send_probe(&mut self, req: &JsonRpcRequest)
        -> Result<JsonRpcResponse, TransportError>;

    async fn send_raw_probe(&mut self, raw: &str) -> Result<Option<String>, TransportError>;

    async fn shutdown(&mut self) -> Result<(), TransportError>;

    fn next_id(&self) -> i64;

    fn protocol_version(&self) -> Option<&str>;

    fn server_info(&self) -> Option<&Implementation>;

    fn classify_error(&self, error: &SessionError) -> AgentErrorEnvelope {
        match error {
            SessionError::JsonRpc { code, message } => AgentErrorEnvelope {
                class: AgentErrorClass::Protocol,
                code: Some(*code),
                message: message.clone(),
            },
            SessionError::Transport(crate::error::TransportError::Timeout(_)) => {
                AgentErrorEnvelope {
                    class: AgentErrorClass::Timeout,
                    code: None,
                    message: error.to_string(),
                }
            }
            SessionError::UnexpectedFormat(msg) => AgentErrorEnvelope {
                class: AgentErrorClass::Validation,
                code: None,
                message: msg.clone(),
            },
            SessionError::Transport(_) => AgentErrorEnvelope {
                class: AgentErrorClass::Transport,
                code: None,
                message: error.to_string(),
            },
            SessionError::NotInitialized => AgentErrorEnvelope {
                class: AgentErrorClass::Unknown,
                code: None,
                message: error.to_string(),
            },
        }
    }
}
