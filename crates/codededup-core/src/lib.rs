//! Core analysis library for `CodeDedup`.
//!
//! Owns the full pipeline: file discovery → tree-sitter parse → normalisation
//! → Merkle fingerprint → clustering → ranking → report. The CLI binary (and
//! a future MCP/LSP daemon) are thin shells over this crate.
//!
//! ## Design invariants
//!
//! - **Byte ranges are canonical**, not line numbers. Line numbers are
//!   computed at render time.
//! - **Global state lives exclusively in [`state`]** — see
//!   [STATE-FILE-REGISTRY].
//! - **Embedding providers are pluggable**; model identity is recorded in
//!   the cache and in every rendered report.
//! - **Incremental updates are a first-class API path**, not a bolt-on.
//!   Batch runs are "incremental starting from an empty cache."

pub mod ast;
pub mod cluster;
pub mod config;
pub mod discover;
pub mod embedding;
pub mod error;
pub mod fingerprint;
pub mod fpcache;
pub mod lang;
pub mod lsh;
pub mod pair;
pub mod pipeline;
pub mod render;
pub mod report;
pub mod sibling;
pub mod state;
pub mod tokens;

pub use config::{ExclusionConfig, DEFAULT_CONFIG_FILENAME};
pub use embedding::{
    EmbeddingMode, EmbeddingProvider, EmbeddingSpec, OllamaProvider, ParseModeError, StubProvider,
    DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};
pub use error::CoreError;
pub use pipeline::{run, EmbeddingSettings, PipelineConfig};
pub use report::{render_report, EmbeddingProvenance, Report, ReportInputs, REPORT_SCHEMA_VERSION};

/// Semantic version of the `codededup-core` library.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
