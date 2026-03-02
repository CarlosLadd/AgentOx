# AgentOx Project Conventions

## Architecture
- Cargo workspace: `agentox-core` (library) + `agentox-cli` (binary)
- agentox-cli is a thin wrapper; all logic lives in agentox-core
- Use `thiserror` for library errors, `anyhow` for binary errors
- All async code uses tokio

## Code Style
- Run `cargo fmt` and `cargo clippy` before commits
- All public types need doc comments
- Error types: one enum per module, derive thiserror::Error
- Tests: unit tests in same file, integration tests in /tests/

## MCP Protocol
- Target spec version: 2025-11-25
- Dual transport: stdio (spawn subprocess) and raw JSON-RPC
- Two client layers: typed (serde structs) and raw (for fuzzing malformed messages)

## Naming
- Check modules: verb_noun pattern (e.g., validate_capabilities)
- Types: PascalCase, no abbreviations except MCP, JSON, RPC
- Check IDs: CONF-001 through CONF-010 (conformance), SEC-001+ (security), BHV-001+ (behavioral)
