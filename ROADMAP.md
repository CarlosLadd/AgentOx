# AgentOx Roadmap

Post-v1.0 roadmap focused on compounding value while preserving the v1 stability promise.

## Principles

- `v1.0.x` is for reliability and developer-experience hardening (no breaking changes).
- `v1.1+` adds capabilities in an additive way.
- `v2.0.0` is reserved for justified breaking changes only.
- HTML reports remain out of scope for this roadmap.

## v1.0.1 (Reliability Patch)

### Goals
- Harden HTTP/SSE GA basic behavior in edge cases.
- Reduce false positives in `SEC-*` and `BHV-*`.
- Improve diagnostics quality for CI failures.

### Planned Work
- Transport retry tuning:
  - configurable retry cap (default unchanged)
  - clearer timeout/network/5xx/4xx classification
- SSE parser robustness:
  - malformed event tolerance
  - clearer diagnostics when `data:` payload is missing/invalid
- Check hardening:
  - reduce noisy outcomes in `SEC-002`, `BHV-002`, `BHV-003`
- Logging clarity:
  - stable stderr wording for top failure classes

### Acceptance Criteria
- No CLI flag contract changes.
- No JSON report field name/type changes.
- Existing pipeline consumers continue to work.
- `fmt`, `test --workspace`, `clippy --workspace --all-targets --all-features -D warnings` pass.

## v1.0.2 (Developer Experience Patch)

### Goals
- Improve CI/local ergonomics while preserving v1 contracts.

### Planned Work
- Add optional non-breaking flags:
  - `--quiet` for minimal stderr output
  - `--output <file>` for report persistence
- Improve command validation and troubleshooting guidance.
- Performance pass:
  - reduce startup overhead
  - remove safe redundant calls

### Acceptance Criteria
- Added flags are optional/backward-compatible.
- JSON report remains schema `1.0`.
- CLI contract docs include CI-focused examples.
- Full quality gates pass.

## v1.1.0 (Additive Capability Expansion)

### Goals
- Add configurable audit execution without changing defaults.

### Planned Work
- Check selection:
  - `--check <ID>` include-list (repeatable)
  - `--skip-check <ID>` exclude-list (repeatable)
- Execution profiles:
  - `--profile quick|standard|full`
- Baseline comparison:
  - compare current JSON report vs previous report for regressions

### Acceptance Criteria
- Default behavior remains identical to `v1.0.x`.
- New options are additive and documented.
- Contract tests verify legacy invocations remain compatible.

## v1.2.0 (Advanced Streaming GA)

### Goals
- Deliver deferred advanced HTTP/SSE streaming lifecycle.

### Planned Work
- Long-lived SSE session management:
  - reconnect policy
  - interruption recovery
  - backoff/jitter strategy
- Event lifecycle support:
  - explicit event-type/heartbeat/partial-frame handling
- Transport observability:
  - stream health counters and diagnostics

### Acceptance Criteria
- `--target` advanced streaming path is GA.
- Dedicated integration coverage for new streaming behavior.
- No breaking changes to stable CLI/JSON contracts.

## v1.3.0 (CI/Ecosystem Integration)

### Goals
- Make AgentOx easier to adopt in production pipelines.

### Planned Work
- Official GitHub Action for audit + artifact upload.
- Publish JSON schema artifact for report contract (`schema_version: "1.0"`).
- Add compatibility examples for common CI providers.

### Acceptance Criteria
- Action supports both stdio and target mode.
- Published JSON schema matches emitted reports.
- Docs include copy-paste integration examples.

## v2.0.0 Entry Criteria (Not Scheduled)

Only move to `v2.0.0` when at least one condition is true:
1. CLI flag/behavior contract must break for architecture reasons.
2. JSON report contract requires incompatible schema changes (`2.x`).
3. Core execution model requires disruptive semantic changes.

Until then, continue shipping on `v1.x`.

## API Stability Policy

- `v1.0.x`:
  - Stable: CLI behavior + JSON report schema `1.0`
  - Non-stable: internal `agentox-core` Rust API
- `v1.1+`:
  - Additive CLI/report features only (no removals/renames)
- `v2.0`:
  - Breaking changes allowed with migration guidance and tooling
