//! Vendor-neutral protocol abstractions and canonical tool models.

pub mod adapter;
pub mod adapters;
pub mod types;

pub use adapter::ProtocolAdapter;
pub use adapters::{A2aProtocolAdapter, McpProtocolAdapter, OpenAiToolUseAdapter};
pub use types::{
    AdapterMetadata, AgentErrorClass, AgentErrorEnvelope, AgentProtocol, AgentToolCallResult,
    AgentToolModel,
};
