//! Pipeline configuration types. Shared between the batch entry
//! point ([`super::run`]) and the incremental session.

use std::path::PathBuf;

use crate::embedding::{EmbeddingMode, EmbeddingProvider};

/// Pipeline-level configuration assembled by the CLI binary.
#[derive(Debug)]
pub struct PipelineConfig<'a> {
    /// Root directory to analyse.
    pub root: PathBuf,
    /// Minimum AST subtree node count considered a clone candidate
    /// (mirrors the CLI `--min-nodes` flag).
    pub min_nodes: u32,
    /// Optional `.deslop.toml` override; `None` means discover in
    /// the scan root.
    pub config_path: Option<PathBuf>,
    /// Embedding-pass configuration ([FUSION-EMBED-PROVIDER]).
    pub embedding: EmbeddingSettings<'a>,
    /// Whether to consult the on-disk fingerprint cache
    /// ([PIPELINE-INCREMENTAL]).
    pub incremental: bool,
}

/// Embedding-pass policy + optional provider.
#[derive(Debug)]
pub struct EmbeddingSettings<'a> {
    /// Resolved `--embeddings` value.
    pub mode: EmbeddingMode,
    /// Borrowed provider. `None` under [`EmbeddingMode::Off`] or when
    /// the CLI decided the provider was unreachable under `auto`.
    pub provider: Option<&'a dyn EmbeddingProvider>,
}
