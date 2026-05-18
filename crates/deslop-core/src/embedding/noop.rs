//! No-op embedding provider used as the placeholder when embeddings
//! are disabled.
//!
//! [`NoopProvider`] is **not** a product provider. The pipeline only
//! invokes provider methods when `EmbeddingMode != Off`; this type
//! exists so the session struct can hold an `Arc<dyn EmbeddingProvider>`
//! field without forcing every caller through an `Option`. Its
//! `provider_id` reports `"off"` to make accidental usage immediately
//! obvious in logs, traces, and reports.
//!
//! The noop is never registered in [`crate::embedding::ProviderRegistry`]
//! and never appears in `embedding/listModels`. Production code that
//! finds Ollama unreachable installs this placeholder and downgrades
//! the session mode to `Off` instead of falling back to the legacy
//! BLAKE3 stub.

use crate::embedding::provider::{EmbeddingProvider, EmbeddingSpec, ProviderError};

/// Reserved id surfaced by [`NoopProvider`]. Never registered as a
/// selectable provider; appears only as the spec on a session that
/// has embeddings disabled.
pub const NOOP_PROVIDER_ID: &str = "off";

/// Placeholder provider used when embeddings are off. Calling
/// [`EmbeddingProvider::embed`] is a programming error — the pipeline
/// short-circuits on `EmbeddingMode::Off` before reaching this code.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl NoopProvider {
    /// Constructs a new noop placeholder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl EmbeddingProvider for NoopProvider {
    fn spec(&self) -> EmbeddingSpec {
        EmbeddingSpec {
            provider_id: NOOP_PROVIDER_ID.to_owned(),
            model_id: NOOP_PROVIDER_ID.to_owned(),
            model_version: NOOP_PROVIDER_ID.to_owned(),
            dimensions: 0,
        }
    }

    fn probe(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn embed(&self, _input: &str) -> Result<Vec<f32>, ProviderError> {
        Err(ProviderError::ProviderFailed {
            provider_id: NOOP_PROVIDER_ID.to_owned(),
            message: "embeddings are disabled; pipeline must short-circuit before calling embed"
                .to_owned(),
        })
    }
}
