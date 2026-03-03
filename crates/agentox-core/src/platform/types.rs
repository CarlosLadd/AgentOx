use crate::checks::types::CheckCategory;
use crate::protocol::mcp_types::{CallToolResult, Content, Tool};
use serde::{Deserialize, Serialize};

/// Supported protocol families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProtocol {
    Mcp,
    A2a,
    OpenAiToolUse,
}

impl std::fmt::Display for AgentProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mcp => write!(f, "mcp"),
            Self::A2a => write!(f, "a2a"),
            Self::OpenAiToolUse => write!(f, "openai_tool_use"),
        }
    }
}

/// Adapter identity metadata emitted in reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterMetadata {
    pub name: String,
    pub version: String,
}

/// Canonical tool definition consumed by protocol-agnostic checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolModel {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
}

impl From<Tool> for AgentToolModel {
    fn from(value: Tool) -> Self {
        Self {
            name: value.name,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            annotations: value.annotations,
        }
    }
}

impl From<AgentToolModel> for Tool {
    fn from(value: AgentToolModel) -> Self {
        Self {
            name: value.name,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            annotations: value.annotations,
        }
    }
}

/// Canonical invocation result wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallResult {
    pub content: Vec<Content>,
    #[serde(default)]
    pub is_error: bool,
}

impl From<CallToolResult> for AgentToolCallResult {
    fn from(value: CallToolResult) -> Self {
        Self {
            content: value.content,
            is_error: value.is_error,
        }
    }
}

impl From<AgentToolCallResult> for CallToolResult {
    fn from(value: AgentToolCallResult) -> Self {
        Self {
            content: value.content,
            is_error: value.is_error,
        }
    }
}

/// Normalized high-level error classes used across protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorClass {
    Protocol,
    Validation,
    Timeout,
    Transport,
    Unknown,
}

/// Protocol-neutral error envelope for diagnostics/evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentErrorEnvelope {
    pub class: AgentErrorClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    pub message: String,
}

/// Report record describing checks skipped for protocol capability mismatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedCheck {
    pub check_id: String,
    pub category: CheckCategory,
    pub reason: String,
}
