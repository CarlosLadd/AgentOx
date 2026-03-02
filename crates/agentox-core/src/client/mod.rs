//! MCP client implementation with stdio and HTTP/SSE transports.

pub mod session;
pub mod stdio;
pub mod transport;

pub use session::McpSession;
pub use stdio::StdioTransport;
pub use transport::Transport;
