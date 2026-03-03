//! Behavioral contract checks (BHV-001+).

mod error_semantics;
mod idempotency;
mod schema_output_alignment;

pub use error_semantics::DeterministicErrorSemantics;
pub use idempotency::IdempotencyBaseline;
pub use schema_output_alignment::SchemaOutputAlignment;
