//! Error types for AgentOx.

/// Errors that can occur during transport operations (sending/receiving messages).
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("no response received from server")]
    NoResponse,

    #[error("server process exited unexpectedly: {0}")]
    ProcessExit(String),

    #[error("operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("command parsing error: {0}")]
    CommandParse(String),
}

/// Errors that can occur during an MCP session.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc { code: i64, message: String },

    #[error("unexpected response format: {0}")]
    UnexpectedFormat(String),

    #[error("server not initialized")]
    NotInitialized,
}
