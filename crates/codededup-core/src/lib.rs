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
pub mod buckets;
pub mod cluster;
pub mod config;
pub mod delta;
pub mod discover;
pub mod embedding;
pub mod error;
pub mod fingerprint;
pub mod fpcache;
pub mod lang;
#[cfg(feature = "live")]
pub mod live;
pub mod lsh;
pub mod pair;
pub mod pipeline;
pub mod render;
pub mod report;
pub mod report_metrics;
pub mod sibling;
pub mod state;
pub mod tokens;

pub use buckets::{bucket_labels, classify, classify_signals, BucketLabels, ClusterKind};
pub use config::{ExclusionConfig, DEFAULT_CONFIG_FILENAME};
pub use delta::ReportDelta;
pub use embedding::{
    list_ollama_models, EmbeddingMode, EmbeddingProvider, EmbeddingSpec, OllamaModelInfo,
    OllamaProvider, ParseModeError, ProviderError, StubProvider, DEFAULT_OLLAMA_ENDPOINT,
    DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};
pub use error::CoreError;
pub use pipeline::{debug_ast_dump, run, EmbeddingSettings, PipelineConfig, PipelineSession};
pub use report::{render_report, EmbeddingProvenance, Report, ReportInputs, REPORT_SCHEMA_VERSION};
pub use report_metrics::{
    compute_repo_metrics, count_analysed_lines, validate_threshold_percent, AnalysedLines,
    MetricsInputs, RepoMetrics, ThresholdSource, ThresholdSummary,
};

/// Semantic version of the `codededup-core` library.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
