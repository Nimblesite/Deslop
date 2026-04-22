//! Pipeline configuration types. Shared between the batch entry
//! point ([`super::run`]) and the incremental session.

use std::{path::PathBuf, time::Duration};

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
pub struct EmbeddingSettings<'a> {
    /// Resolved `--embeddings` value.
    pub mode: EmbeddingMode,
    /// Borrowed provider. `None` under [`EmbeddingMode::Off`] or when
    /// the CLI decided the provider was unreachable under `auto`.
    pub provider: Option<&'a dyn EmbeddingProvider>,
    /// Optional low-priority yield between provider batches. Live
    /// surfaces set this so embedding work does not monopolise CPU.
    pub batch_yield: Option<Duration>,
    /// Optional progress sink called after each provider batch.
    pub progress: Option<&'a dyn Fn(usize)>,
}

impl std::fmt::Debug for EmbeddingSettings<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingSettings")
            .field("mode", &self.mode)
            .field("provider", &self.provider.is_some())
            .field("batch_yield", &self.batch_yield)
            .field("progress", &self.progress.is_some())
            .finish()
    }
}
