# AgentOx Roadmap

Post-v1.0 roadmap to position AgentOx as a vendor-neutral agentic tool security platform across MCP, A2A, and OpenAI tool_use.

## Positioning Goal

- Primary buyer: Dev teams (CI/CD-first adoption)
- Product moat: policy-as-code + portable evidence artifacts
- Delivery strategy: MCP parity baseline first, then staged multi-protocol GA
- HTML reports: out of scope

## Release Train

## v1.0.1 (Reliability Patch)

### Goals
- Harden HTTP/SSE GA basic behavior in real-world edge cases.
- Reduce noisy outcomes in `SEC-*` and `BHV-*`.
- Improve CI diagnostics quality.

### Planned Work
- Retry tuning and clearer timeout/network/4xx/5xx classification.
- SSE parser robustness for malformed/empty `data:` payloads.
- Check hardening in `SEC-002`, `BHV-002`, `BHV-003`.
- Stable stderr wording for top failure classes.

### Acceptance Criteria
- No CLI contract changes.
- No JSON field name/type changes.
- Full gates pass (`fmt`, `test --workspace`, `clippy -D warnings`).

## v1.0.2 (Developer Experience Patch)

### Goals
- Improve operator ergonomics without breaking v1 contracts.

### Planned Work
- Add optional flags: `--quiet`, `--output <file>`.
- Better validation/troubleshooting messages.
- Startup/performance cleanup (safe redundant-call reduction).

### Acceptance Criteria
- Backward-compatible CLI behavior.
- JSON report remains `schema_version: "1.0"`.
- Updated CI-focused docs and examples.

## v1.1.0 (Platform Core Extraction)

### Goals
- Decouple checks from MCP-specific session/method assumptions.

### Planned Work
- Canonical model for tools/invocation/error envelopes.
- Protocol adapter contract and MCP adapter parity.
- Move reusable checks to protocol-agnostic paths.
- Keep MCP-specific conformance checks in protocol namespace.

### Acceptance Criteria
- MCP behavior/check IDs unchanged from user perspective.
- Adapter-backed execution powers default audits.
- No regression in existing MCP reports.

## v1.2.0 (Policy + Evidence Moat)

### Goals
- Add defensible policy/evidence capabilities that vendor CLIs typically lack.

### Planned Work
- `agentox-policy.yaml` policy bundles (thresholds, targeting, env overrides).
- Policy evaluation mode (`--policy`) with pass/warn/fail semantics.
- Baseline regression guard (`--baseline`) for new failures.
- Deterministic evidence signature metadata in JSON output.

### Acceptance Criteria
- Teams can enforce policy consistently in CI.
- Evidence artifacts are deterministic and verifiable.
- JSON schema remains additive/backward-compatible.

## v1.3.0 (A2A Adapter GA)

### Goals
- Ship first non-MCP protocol as GA.

### Planned Work
- Full `A2aAdapter` mapping to canonical model.
- Protocol capability matrix in reports (`protocol`, `adapter`, `unsupported_checks`).
- Reuse protocol-agnostic `SEC-*`/`BHV-*`; add `CONF-A2A-*` where necessary.

### Acceptance Criteria
- `--target` with A2A endpoint produces complete audit + policy decision.
- Cross-protocol summary semantics match MCP path.

## v1.4.0 (OpenAI tool_use Adapter GA)

### Goals
- Complete MCP + A2A + OpenAI tool_use platform coverage.

### Planned Work
- `OpenAiToolUseAdapter` with canonical normalization.
- Protocol profiles (`mcp-strict`, `a2a-standard`, `openai-tool-use`).
- Cross-protocol CI and policy templates.

### Acceptance Criteria
- Same policy bundle runs across all three protocols.
- Same report schema and gate semantics across adapters.

## v2.0.0 Entry Criteria (Not Scheduled)

Move to `v2.0.0` only if at least one is true:
1. CLI flag/behavior contract must break for architecture reasons.
2. JSON report contract requires incompatible schema changes (`2.x`).
3. Core execution semantics require disruptive changes.

Until then, continue shipping in `v1.x` with additive compatibility.

## API Stability Policy

- `v1.0.x`: stable CLI behavior + JSON report schema `1.0`.
- `v1.1+`: additive CLI/report capabilities only (no removals/renames).
- Internal `agentox-core` APIs remain non-stable unless explicitly frozen.
- `v2.0`: breaking changes allowed with migration guidance.
