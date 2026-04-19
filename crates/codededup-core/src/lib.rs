//! Core analysis library for `CodeDedup`.
//!
//! Owns the full pipeline: file discovery → tree-sitter parse → normalization
//! → Merkle fingerprint → clustering → token LSH → embeddings → fusion →
//! ranking → report. The CLI binary (and a future MCP/LSP daemon) are thin
//! shells over this crate.
//!
//! ## Design invariants
//!
//! - **Byte ranges are canonical**, not line numbers. Line numbers are
//!   computed at render time.
//! - **Global state lives exclusively in [`state`]**.
//! - **Embedding providers are pluggable**; model identity is recorded in the
//!   cache and in every rendered report.
//! - **Incremental updates are a first-class API path**, not a bolt-on. Batch
//!   runs are "incremental starting from an empty cache."

pub mod state;

/// Semantic version of the `codededup-core` library.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
