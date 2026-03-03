//! Check runner — orchestrates check execution against an MCP session.

use crate::checks::types::{CheckCategory, CheckResult};
use crate::client::{HttpSseTransport, McpSession, StdioTransport};
use crate::protocol::mcp_types::{InitializeResult, Tool};
use std::time::Duration;

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
    /// Original connection target used to create reconnectable disposable sessions.
    pub target: ConnectionTarget,
    /// Parsed initialize result.
    pub init_result: Option<InitializeResult>,
    /// Raw initialize response string.
    pub raw_init_response: Option<String>,
    /// Cached tools list.
    pub tools: Option<Vec<Tool>>,
    /// Per-request transport timeout.
    pub request_timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum ConnectionTarget {
    Stdio { command: String },
    HttpSse { endpoint: String },
}

impl CheckContext {
    /// Create a new check context.
    pub fn new(session: McpSession, target: ConnectionTarget) -> Self {
        Self {
            session,
            target,
            init_result: None,
            raw_init_response: None,
            tools: None,
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Spawn a fresh session without initializing it.
    pub async fn fresh_session(&self) -> Result<McpSession, crate::error::SessionError> {
        match &self.target {
            ConnectionTarget::Stdio { command } => {
                let mut transport = StdioTransport::spawn_quiet(command)
                    .await
                    .map_err(crate::error::SessionError::Transport)?;
                transport.set_read_timeout(self.request_timeout);
                Ok(McpSession::new(Box::new(transport)))
            }
            ConnectionTarget::HttpSse { endpoint } => {
                let transport = HttpSseTransport::new(endpoint.clone(), self.request_timeout);
                Ok(McpSession::new(Box::new(transport)))
            }
        }
    }

    /// Spawn a fresh disposable session for destructive tests.
    /// The caller is responsible for shutting it down.
    /// Uses `spawn_quiet` to suppress server stderr noise.
    pub async fn disposable_session(&self) -> Result<McpSession, crate::error::SessionError> {
        let mut session = self.fresh_session().await?;
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

    /// Register all default security checks.
    pub fn register_security_checks(&mut self) {
        use crate::checks::security::*;
        self.register(Box::new(PromptInjectionEchoSafety));
        self.register(Box::new(ToolParameterBoundaryValidation));
        self.register(Box::new(ErrorLeakageDetection));
        self.register(Box::new(ResourceExhaustionGuardrail));
    }

    /// Register behavioral checks (reserved for future versions).
    pub fn register_behavioral_checks(&mut self) {
        use crate::checks::behavioral::*;
        self.register(Box::new(IdempotencyBaseline));
        self.register(Box::new(SchemaOutputAlignment));
        self.register(Box::new(DeterministicErrorSemantics));
    }

    /// Register default checks for v0.4 (conformance + security + behavioral).
    pub fn register_default_v0_4_checks(&mut self) {
        self.register_conformance_checks();
        self.register_security_checks();
        self.register_behavioral_checks();
    }

    /// Get the total number of registered checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Run all registered checks and return all results.
    pub async fn run_all(&self, ctx: &mut CheckContext) -> Vec<CheckResult> {
        self.run_all_with_progress(ctx, |_, _, _| {}).await
    }

    /// Run all registered checks with a progress callback.
    ///
    /// The callback is invoked after each check completes with:
    /// - `check_id`: the ID of the check that just finished (e.g., "CONF-001")
    /// - `check_name`: the human-readable name
    /// - `results`: the results produced by this check
    pub async fn run_all_with_progress<F>(
        &self,
        ctx: &mut CheckContext,
        mut on_check_done: F,
    ) -> Vec<CheckResult>
    where
        F: FnMut(&str, &str, &[CheckResult]),
    {
        let mut results = Vec::new();
        for check in &self.checks {
            tracing::info!(check_id = %check.id(), name = %check.name(), "running check");
            let start = std::time::Instant::now();
            let mut check_results = check.run(ctx).await;
            let elapsed = start.elapsed().as_millis() as u64;
            for r in &mut check_results {
                r.duration_ms = elapsed;
            }
            on_check_done(check.id(), check.name(), &check_results);
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
