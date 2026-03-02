//! Check runner — orchestrates check execution against an MCP session.

use crate::checks::types::{CheckCategory, CheckResult};
use crate::client::stdio::StdioTransport;
use crate::client::McpSession;
use crate::protocol::mcp_types::{InitializeResult, Tool};

/// Trait that all audit checks implement.
#[async_trait::async_trait]
pub trait Check: Send + Sync {
    /// Unique check ID (e.g., "CONF-001").
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Category of this check.
    fn category(&self) -> CheckCategory;

    /// Run the check. May return multiple findings.
    async fn run(&self, ctx: &mut CheckContext) -> Vec<CheckResult>;
}

/// Context provided to checks during execution.
pub struct CheckContext {
    /// The active MCP session.
    pub session: McpSession,
    /// The original command used to spawn the server (for reconnection).
    pub command: String,
    /// Parsed initialize result.
    pub init_result: Option<InitializeResult>,
    /// Raw initialize response string.
    pub raw_init_response: Option<String>,
    /// Cached tools list.
    pub tools: Option<Vec<Tool>>,
}

impl CheckContext {
    /// Create a new check context.
    pub fn new(session: McpSession, command: String) -> Self {
        Self {
            session,
            command,
            init_result: None,
            raw_init_response: None,
            tools: None,
        }
    }

    /// Spawn a fresh disposable session for destructive tests.
    /// The caller is responsible for shutting it down.
    pub async fn disposable_session(&self) -> Result<McpSession, crate::error::SessionError> {
        let transport = StdioTransport::spawn(&self.command)
            .await
            .map_err(crate::error::SessionError::Transport)?;
        let mut session = McpSession::new(Box::new(transport));
        session.initialize().await?;
        Ok(session)
    }
}

/// Runs a set of checks against a session.
pub struct CheckRunner {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRunner {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a single check.
    pub fn register(&mut self, check: Box<dyn Check>) {
        self.checks.push(check);
    }

    /// Register all default conformance checks.
    pub fn register_conformance_checks(&mut self) {
        use crate::checks::conformance::*;
        self.register(Box::new(InitializeCapabilities));
        self.register(Box::new(JsonRpcStructure));
        self.register(Box::new(ToolsListValid));
        self.register(Box::new(ToolInputSchemaValid));
        self.register(Box::new(MalformedRequestHandling));
        self.register(Box::new(UnknownMethodHandling));
        self.register(Box::new(ErrorCodeCorrectness));
        self.register(Box::new(CapabilityNegotiation));
        self.register(Box::new(ProtocolVersionValidation));
        self.register(Box::new(InitializedNotificationOrder));
    }

    /// Get the total number of registered checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Run all registered checks and return all results.
    pub async fn run_all(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        let mut results = Vec::new();
        for check in &self.checks {
            tracing::info!(check_id = %check.id(), name = %check.name(), "running check");
            let start = std::time::Instant::now();
            let mut check_results = check.run(ctx).await;
            let elapsed = start.elapsed().as_millis() as u64;
            for r in &mut check_results {
                r.duration_ms = elapsed;
            }
            results.extend(check_results);
        }
        results
    }
}

impl Default for CheckRunner {
    fn default() -> Self {
        Self::new()
    }
}
