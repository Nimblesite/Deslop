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
//!
//! The deterministic BLAKE3 shim formerly known as `StubProvider`
//! lives under [`test_support`] now and is gated behind the
//! `test-support` Cargo feature so it never ships in the production
//! VSIX, LSP, or MCP binaries.

pub mod cache;
pub mod mode;
pub mod noop;
pub mod ollama;
pub mod pairs;
pub mod provider;
pub mod registry;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use cache::{content_hash, EmbeddingCache};
pub use mode::{EmbeddingMode, ParseModeError};
pub use noop::{NoopProvider, NOOP_PROVIDER_ID};
pub use ollama::{
    list_models as list_ollama_models, OllamaModelInfo, OllamaProvider, DEFAULT_OLLAMA_ENDPOINT,
    DEFAULT_OLLAMA_MODEL,
};
pub use pairs::{embedding_pairs, EmbeddingPair};
pub use provider::{
    EmbeddingProvider, EmbeddingSpec, ProviderError, DEFAULT_MAX_INPUT_CHARS, DEFAULT_PROVIDER_ID,
};
pub use registry::{ProviderRegistry, RegistryError};

use std::sync::Arc;

/// Attempts to connect to Ollama. Returns `Some(provider)` when Ollama
/// is reachable and `None` otherwise so callers can fall through to
/// the "no embeddings" code path without crash-looping.
///
/// For interactive server processes (LSP, MCP) embeddings are optional
/// per [LSP-EMBEDDING-CONSENT] / issue #35 — the server must stay alive
/// regardless of mode. The caller decides how to log the failure
/// (`error` for `Required`, `warn` for `Auto`).
#[must_use]
pub fn try_connect_ollama(endpoint: &str, model: &str) -> Option<Arc<dyn EmbeddingProvider>> {
    match OllamaProvider::connect(endpoint, model) {
        Ok(provider) => Some(Arc::new(provider)),
        Err(err) => {
            tracing::warn!(%err, endpoint, model, "ollama_unreachable");
            None
        }
    }
}
