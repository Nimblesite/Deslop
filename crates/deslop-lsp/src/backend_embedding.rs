//! Startup embedding configuration shared by the LSP backend.
use std::sync::Arc;

use deslop_core::{
    embedding::{
        EmbeddingMode, EmbeddingProvider, NoopProvider, ProviderRegistry, RegistryError,
        DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID,
    },
    live::LiveError,
};

/// [LSP-EMBEDDING-CONSENT] Embedding startup settings supplied by the client after the user
/// has explicitly selected a model. `Off` means no startup embedding
/// pass runs.
#[derive(Debug, Clone)]
pub struct LspEmbeddingConfig {
    /// Live embedding mode.
    pub mode: EmbeddingMode,
    /// Provider registry key.
    pub provider_id: String,
    /// Model id.
    pub model_id: String,
    /// Provider endpoint.
    pub endpoint: String,
}

impl Default for LspEmbeddingConfig {
    fn default() -> Self {
        Self {
            mode: EmbeddingMode::Off,
            provider_id: DEFAULT_PROVIDER_ID.to_owned(),
            model_id: DEFAULT_OLLAMA_MODEL.to_owned(),
            endpoint: DEFAULT_OLLAMA_ENDPOINT.to_owned(),
        }
    }
}

/// Resolves the startup `(provider, mode)` pair for the LSP backend.
///
/// For `EmbeddingMode::Off` the LSP installs a [`NoopProvider`] and
/// keeps mode `Off` — embeddings stay disabled until the user picks a
/// model. For `Auto` / `Required` we ask the production
/// [`ProviderRegistry`] for the requested provider. When the provider
/// is unreachable, we install [`NoopProvider`] and downgrade the mode
/// to `Off` so the editor keeps working without semantic recall — the
/// LSP must never crash-loop VS Code per. The log level
/// reflects intent: `error` when the user opted into `Required` and
/// `warn` when `Auto` silently degraded.
pub(super) fn resolve_startup_provider(
    embedding: &LspEmbeddingConfig,
) -> Result<(Arc<dyn EmbeddingProvider>, EmbeddingMode), LiveError> {
    if matches!(embedding.mode, EmbeddingMode::Off) {
        return Ok((Arc::new(NoopProvider::new()), EmbeddingMode::Off));
    }
    let registry = ProviderRegistry::production();
    match registry.build(
        &embedding.provider_id,
        &embedding.model_id,
        Some(&embedding.endpoint),
    ) {
        Ok(provider) => Ok((provider, embedding.mode)),
        Err(RegistryError::Unsupported {
            requested,
            registered,
        }) => Err(LiveError::UnsupportedProvider {
            requested,
            registered,
        }),
        Err(RegistryError::Provider(provider_error)) => {
            log_provider_unreachable(embedding, &provider_error);
            Ok((Arc::new(NoopProvider::new()), EmbeddingMode::Off))
        }
    }
}

/// Emits the appropriate log when the configured provider is not
/// reachable. `Required` users opted in explicitly so the failure is
/// surfaced at `error`; `Auto` users get a `warn`.
fn log_provider_unreachable(
    embedding: &LspEmbeddingConfig,
    error: &deslop_core::embedding::ProviderError,
) {
    if matches!(embedding.mode, EmbeddingMode::Required) {
        tracing::error!(
            %error,
            endpoint = %embedding.endpoint,
            model = %embedding.model_id,
            "lsp_embedding_required_provider_unreachable",
        );
    } else {
        tracing::warn!(
            %error,
            endpoint = %embedding.endpoint,
            model = %embedding.model_id,
            "lsp_embedding_auto_provider_unreachable",
        );
    }
}
