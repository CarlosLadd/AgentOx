//! Transport trait for sending/receiving raw JSON-RPC messages.

use crate::error::TransportError;
use crate::protocol::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// Transport capabilities used by checks and future transport-specific logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportCapabilities {
    pub request_response: bool,
    pub streaming_notifications: bool,
}

/// Trait for sending/receiving raw JSON-RPC messages over any transport.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Capability metadata for this transport implementation.
    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            request_response: true,
            streaming_notifications: false,
        }
    }

    /// Write a raw message string without reading a response.
    /// Used for notifications and other one-way messages.
    async fn write_raw(&mut self, message: &str) -> Result<(), TransportError>;

    /// Write a raw message string and read back one response line.
    /// Returns `None` if the response line is empty.
    async fn request_raw(&mut self, message: &str) -> Result<Option<String>, TransportError>;

    /// Send a typed JSON-RPC request and get the parsed response.
    async fn send_request(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        let raw = serde_json::to_string(req)?;
        let response_str = self
            .request_raw(&raw)
            .await?
            .ok_or(TransportError::NoResponse)?;
        let response: JsonRpcResponse = serde_json::from_str(&response_str)?;
        Ok(response)
    }

    /// Send a JSON-RPC notification (write-only, no response expected).
    async fn send_notification(
        &mut self,
        notif: &JsonRpcNotification,
    ) -> Result<(), TransportError> {
        let raw = serde_json::to_string(notif)?;
        self.write_raw(&raw).await
    }

    /// Send a raw string and read one response. Convenience alias for the fuzzer.
    async fn send_raw(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        self.request_raw(message).await
    }

    /// Shut down the transport gracefully.
    async fn shutdown(&mut self) -> Result<(), TransportError>;
}
