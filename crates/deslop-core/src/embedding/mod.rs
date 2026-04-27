//! Embedding pass ([TECH-EMBED-NEURAL] / [FUSION-EMBED-PROVIDER]).
//!
//! Pluggable `EmbeddingProvider` trait, a disk cache keyed by
//! `(content_hash, provider_id, model_id, model_version)`, and an HNSW
//! top-k pair generator that produces the `embedding_cos` signal
//! consumed by [FUSION-STRATEGY-MAX-SUM].
//!
//! The module is deliberately small: the trait is the extension point
//! per [PIPELINE-LANG-TRAIT]-style "single surface" design, and every
//! other file in this module is a concrete collaborator implementing
//! that surface (provider implementation, on-disk cache, ANN index).

pub mod cache;
pub mod mode;
pub mod ollama;
pub mod pairs;
pub mod provider;
pub mod stub;

pub use cache::{content_hash, EmbeddingCache};
pub use mode::{EmbeddingMode, ParseModeError};
pub use ollama::{
    list_models as list_ollama_models, OllamaModelInfo, OllamaProvider, DEFAULT_OLLAMA_ENDPOINT,
    DEFAULT_OLLAMA_MODEL,
};
pub use pairs::{embedding_pairs, EmbeddingPair};
pub use provider::{EmbeddingProvider, EmbeddingSpec, ProviderError, DEFAULT_PROVIDER_ID};
pub use stub::{StubProvider, PROVIDER_ID as STUB_PROVIDER_ID};

use std::sync::Arc;

/// Attempts to connect to Ollama, always returning a usable provider.
///
/// For interactive server processes (LSP, MCP) embeddings must never
/// cause a crash-loop — the server must stay alive regardless of mode
/// ([LSP-EMBEDDING-CONSENT], issue #35). `Required` logs at error level
/// so the user knows their explicit opt-in was not fulfilled; `Auto`
/// logs at warn. The CLI batch tool enforces hard-fail semantics via its
/// own code path where `Required` is genuinely terminal.
pub fn connect_or_stub(mode: EmbeddingMode, endpoint: &str, model: &str) -> Arc<dyn EmbeddingProvider> {
    match OllamaProvider::connect(endpoint, model) {
        Ok(provider) => Arc::new(provider),
        Err(err) if matches!(mode, EmbeddingMode::Required) => {
            tracing::error!(
                %err,
                endpoint,
                model,
                "ollama_unreachable_required_mode_falling_back_to_stub"
            );
            Arc::new(StubProvider::new())
        }
        Err(err) => {
            tracing::warn!(
                %err,
                endpoint,
                model,
                "ollama_unreachable_falling_back_to_stub"
            );
            Arc::new(StubProvider::new())
        }
    }
}
