//! MCP session management — handles the protocol handshake and method calls.

use crate::client::transport::Transport;
use crate::error::{SessionError, TransportError};
use crate::protocol::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::protocol::mcp_types::*;
use std::sync::atomic::{AtomicI64, Ordering};

/// An MCP session that manages the protocol lifecycle over a transport.
pub struct McpSession {
    transport: Box<dyn Transport>,
    next_id: AtomicI64,
    server_capabilities: Option<ServerCapabilities>,
    server_info: Option<Implementation>,
    protocol_version: Option<String>,
}

impl McpSession {
    /// Create a new session wrapping a transport.
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            next_id: AtomicI64::new(1),
            server_capabilities: None,
            server_info: None,
            protocol_version: None,
        }
    }

    /// Perform the MCP initialize handshake.
    pub async fn initialize(&mut self) -> Result<InitializeResult, SessionError> {
        let params = InitializeParams {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "agentox".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
        };

        let response = self.call("initialize", &params).await?;
        let result: InitializeResult = serde_json::from_value(response).map_err(|e| {
            SessionError::UnexpectedFormat(format!("invalid initialize result: {e}"))
        })?;

        // Send the initialized notification
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        self.transport
            .send_notification(&notif)
            .await
            .map_err(SessionError::Transport)?;

        // Store server info
        self.server_capabilities = Some(result.capabilities.clone());
        self.server_info = Some(result.server_info.clone());
        self.protocol_version = Some(result.protocol_version.clone());

        tracing::info!(
            server = %result.server_info.name,
            version = ?result.server_info.version,
            protocol = %result.protocol_version,
            "MCP session initialized"
        );

        Ok(result)
    }

    /// List all tools, following pagination cursors.
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, SessionError> {
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = match &cursor {
                Some(c) => serde_json::json!({ "cursor": c }),
                None => serde_json::json!({}),
            };

            let response = self.call("tools/list", &params).await?;
            let result: ListToolsResult = serde_json::from_value(response).map_err(|e| {
                SessionError::UnexpectedFormat(format!("invalid tools/list result: {e}"))
            })?;

            all_tools.extend(result.tools);

            match result.next_cursor {
                Some(next) if !next.is_empty() => cursor = Some(next),
                _ => break,
            }
        }

        Ok(all_tools)
    }

    /// Call a specific tool.
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, SessionError> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };
        let response = self.call("tools/call", &params).await?;
        let result: CallToolResult = serde_json::from_value(response).map_err(|e| {
            SessionError::UnexpectedFormat(format!("invalid tools/call result: {e}"))
        })?;
        Ok(result)
    }

    /// Send a raw string message (bypasses all type checking). Used for fuzzing.
    pub async fn send_raw(&mut self, raw: &str) -> Result<Option<String>, TransportError> {
        self.transport.send_raw(raw).await
    }

    /// Send a typed request and get the raw JSON-RPC response.
    pub async fn send_request(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        self.transport.send_request(req).await
    }

    /// Get the server capabilities (available after initialize).
    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    /// Get the server info (available after initialize).
    pub fn server_info(&self) -> Option<&Implementation> {
        self.server_info.as_ref()
    }

    /// Get the negotiated protocol version.
    pub fn protocol_version(&self) -> Option<&str> {
        self.protocol_version.as_deref()
    }

    /// Shut down the session and underlying transport.
    pub async fn shutdown(&mut self) -> Result<(), TransportError> {
        self.transport.shutdown().await
    }

    /// Get the next request ID.
    pub fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a typed method call and return the result value.
    async fn call<P: serde::Serialize>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<serde_json::Value, SessionError> {
        let id = self.next_id();
        let params_value = serde_json::to_value(params)
            .map_err(|e| SessionError::UnexpectedFormat(e.to_string()))?;

        let req = JsonRpcRequest::new(id, method, Some(params_value));
        let response = self
            .transport
            .send_request(&req)
            .await
            .map_err(SessionError::Transport)?;

        if let Some(error) = response.error {
            return Err(SessionError::JsonRpc {
                code: error.code,
                message: error.message,
            });
        }

        response.result.ok_or_else(|| {
            SessionError::UnexpectedFormat("response has neither result nor error".to_string())
        })
    }
}
