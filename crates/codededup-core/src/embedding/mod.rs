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
pub use ollama::{OllamaProvider, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL};
pub use pairs::{embedding_pairs, EmbeddingPair};
pub use provider::{EmbeddingProvider, EmbeddingSpec, ProviderError, DEFAULT_PROVIDER_ID};
pub use stub::{StubProvider, PROVIDER_ID as STUB_PROVIDER_ID};
