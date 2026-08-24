//! Core analysis library for `Deslop`.
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
pub mod boilerplate;
pub mod buckets;
pub mod clone_category;
pub mod cluster;
mod cluster_filters;
pub mod config;
pub mod content;
pub mod delta;
pub mod diff_scope;
pub mod discover;
pub mod embedding;
pub mod error;
pub mod fingerprint;
pub mod fpcache;
pub mod lang;
#[cfg(feature = "live")]
pub mod live;
pub mod lsh;
pub mod observe;
pub mod overlap;
pub mod signature_arena;
pub mod pair;
pub mod paths;
pub mod pipeline;
pub mod process;
pub mod refactor;
pub mod render;
pub mod report;
pub mod report_boilerplate;
#[cfg(any(test, feature = "test-support"))]
pub mod report_fixtures;
pub mod report_hints;
pub mod report_location;
pub mod report_metrics;
mod report_render;
pub mod report_restamp;
mod report_weight;
pub mod sibling;
pub mod state;
pub mod tokens;
pub mod version_contract;
/// Wire-format models generated from `docs/models/live-ipc.td` by
/// `scripts/typediagram/generate.mjs`. Always compiled (no feature gate)
/// because the always-on `embedding::ollama` module re-exports
/// `OllamaModelInfo` from here.
pub mod wire_generated;

pub use buckets::{bucket_labels, classify, classify_signals, BucketLabels, ClusterKind};
pub use clone_category::CloneCategory;
pub use config::{
    BoilerplateImportsMode, ClonePolicy, ExclusionConfig, RankingPolicy, DEFAULT_CONFIG_FILENAME,
};
pub use delta::ReportDelta;
pub use diff_scope::{apply_only_changed, parse_unified_diff, DiffScope, ParsedDiff};
pub use embedding::{
    list_ollama_models, EmbeddingMode, EmbeddingProvider, EmbeddingSpec, NoopProvider,
    OllamaModelInfo, OllamaProvider, ParseModeError, ProviderError, ProviderRegistry,
    RegistryError, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID,
    NOOP_PROVIDER_ID,
};
pub use error::CoreError;
pub use pipeline::{debug_ast_dump, run, EmbeddingSettings, PipelineConfig, PipelineSession};
pub use cluster_filters::ParseCache;
pub use report::{render_report, EmbeddingProvenance, Report, ReportInputs};
pub use report_boilerplate::{ReportBoilerplateHint, ReportBoilerplateOccurrence};
pub use report_metrics::{
    compute_repo_metrics, count_analysed_lines, validate_threshold_percent, AnalysedLines,
    MetricsInputs, RepoMetrics, ThresholdSource, ThresholdSummary,
};
pub use version_contract::{
    json_version_line, plain_version_line, requests_version, version_contract_output, ComponentKind,
};

/// Semantic version of the `deslop-core` library.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
