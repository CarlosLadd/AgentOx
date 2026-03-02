//! AgentOx Core — audit engine for MCP server conformance, security, and behavior.
//!
//! This library provides the core audit logic that powers the `agentox` CLI tool.
//! It can also be embedded in other tools that need to audit MCP servers programmatically.

pub mod checks;
pub mod client;
pub mod error;
pub mod protocol;
pub mod report;
