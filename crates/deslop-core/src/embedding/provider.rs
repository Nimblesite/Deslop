//! `EmbeddingProvider` trait and supporting types.
//!
//! Implements [FUSION-EMBED-PROVIDER]: the core surface every embedding
//! backend implements so the pipeline stays agnostic of *how* vectors
//! are produced. A provider reports three identity fields —
//! `provider_id` (short registry key, e.g. `"ollama"`), `model_id`
//! (human-readable model name), and `model_version` (opaque version
//! string) — all three are written into the cache header and the
//! rendered report so that switching models invalidates only the
//! embedding layer deterministically.

use thiserror::Error;

/// Identity triple and dimensionality of the embeddings produced by
/// a provider. Surfaces the values the cache and report keep.
#[derive(Debug, Clone)]
pub struct EmbeddingSpec {
    /// Short registry key, e.g. `"ollama"`.
    pub provider_id: String,
    /// Human-readable model name, e.g. `"nomic-embed-text"`.
    pub model_id: String,
    /// Opaque version string reported by the provider. Any change
    /// invalidates the embedding cache.
    pub model_version: String,
    /// Embedding dimensionality. Reported so consumers can defensively
    /// reject mixed-dim caches.
    pub dimensions: usize,
}

/// Default provider registry key.
pub const DEFAULT_PROVIDER_ID: &str = "ollama";

/// Conservative per-input character budget assumed for any provider
/// that does not declare its own ([`EmbeddingProvider::max_input_chars`]).
pub const DEFAULT_MAX_INPUT_CHARS: usize = 6_000;

/// Errors surfaced by an [`EmbeddingProvider`] implementation.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The provider could not be reached at all (connection refused,
    /// DNS failure, etc.). Propagated to the pipeline so
    /// `--embeddings=required` can hard-fail and `--embeddings=auto`
    /// can fall back with a warning.
    #[error("embedding provider {provider_id} unreachable: {message}")]
    Unreachable {
        /// Provider registry key that failed.
        provider_id: String,
        /// Human-readable reason (transport error, timeout, etc.).
        message: String,
    },
    /// The provider was reached but returned an error response.
    #[error("embedding provider {provider_id} returned an error: {message}")]
    ProviderFailed {
        /// Provider registry key that returned an error.
        provider_id: String,
        /// Upstream error message (HTTP status + body excerpt).
        message: String,
    },
    /// The provider returned a response that could not be parsed or
    /// whose dimensionality differs from the advertised spec.
    #[error("embedding provider {provider_id} returned malformed output: {message}")]
    Malformed {
        /// Provider registry key.
        provider_id: String,
        /// Details of the malformation.
        message: String,
    },
}

/// Pluggable embedding backend. See module docs for the contract.
pub trait EmbeddingProvider: std::fmt::Debug + Send + Sync {
    /// Returns the identity + dimensionality of this provider. Must be
    /// stable for the lifetime of the value.
    fn spec(&self) -> EmbeddingSpec;

    /// Probes the provider for reachability without producing an
    /// embedding. Returns `Ok(())` when the provider is ready to serve
    /// requests. Used by `--embeddings=auto` to decide whether to fall
    /// back and by `--embeddings=required` to fail fast.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Unreachable`] when the provider is not
    /// reachable or [`ProviderError::ProviderFailed`] when reachable
    /// but not ready.
    fn probe(&self) -> Result<(), ProviderError>;

    /// Embeds `input` and returns a dense vector. Length must match
    /// [`EmbeddingSpec::dimensions`].
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] when the provider cannot be reached,
    /// returns an error, or returns a malformed response.
    fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError>;

    /// Maximum number of inputs the provider is willing to embed in a
    /// single call to [`EmbeddingProvider::embed_batch`].
    fn max_batch_size(&self) -> usize {
        1
    }

    /// Maximum source characters this provider accepts in one input.
    /// Subtrees longer than this are counted as failures and never
    /// dispatched ([FUSION-EMBED-PROVIDER], #82).
    ///
    /// The budget belongs to the provider because it is a property of
    /// the model behind it — `nomic-embed-text` reports a 2,048-token
    /// context, `mxbai-embed-large` only 512. A pipeline-wide constant
    /// cannot be right for both: too generous and the provider
    /// silently truncates, too tight and the largest subtrees — the
    /// ones re-derived duplication hurts most — are dropped from the
    /// index with only `failed_subtrees` to show for it (#286).
    ///
    /// The conservative default is what every provider got before the
    /// budget became overridable; implementations that know their
    /// model's real capacity should report it.
    fn max_input_chars(&self) -> usize {
        DEFAULT_MAX_INPUT_CHARS
    }

    /// Embeds multiple inputs in one provider call. Providers that do
    /// not support batching inherit the single-input fallback.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] when the provider cannot produce the
    /// whole batch. The pipeline treats that as every input in the
    /// batch failing and continues with other batches.
    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        inputs.iter().map(|input| self.embed(input)).collect()
    }
}
