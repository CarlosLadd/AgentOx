//! MCP client implementation with stdio and HTTP/SSE transports.

#[cfg(feature = "http-sse")]
pub mod http_sse;
pub mod session;
pub mod stdio;
pub mod transport;

#[cfg(feature = "http-sse")]
pub use http_sse::HttpSseTransport;
pub use session::McpSession;
pub use stdio::StdioTransport;
pub use transport::Transport;
