//! Provider selection and embedding refresh logic for the MCP backend.

use std::sync::{Arc, Mutex};

use deslop_core::{
    EmbeddingMode, EmbeddingProvider, EmbeddingSettings, PipelineSession, StubProvider,
    DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};
use tracing::{info, warn};

use crate::{notify::push_report_changed, NotificationSender};

use super::{live_batch_yield, pipeline::SessionState, BackendError, SessionBackendConfig};

/// Resolves the configured provider using the `embedding_mode` /
/// `embedding_provider` / `embedding_model` / `embedding_endpoint`
/// tuple. Mirrors the CLI's provider selection so MCP sessions match
/// batch runs exactly.
pub(super) fn select_provider(
    config: &SessionBackendConfig,
) -> Result<Option<Arc<dyn EmbeddingProvider>>, BackendError> {
    match config.embedding_mode {
        EmbeddingMode::Off => Ok(None),
        EmbeddingMode::Auto | EmbeddingMode::Required => match config.embedding_provider.as_str() {
            STUB_PROVIDER_ID => Ok(Some(Arc::new(StubProvider::new()))),
            DEFAULT_PROVIDER_ID => Ok(Some(deslop_core::embedding::connect_or_stub(
                config.embedding_mode,
                &config.embedding_endpoint,
                &config.embedding_model,
            ))),
            other => Err(BackendError::UnknownEmbeddingProvider(other.to_owned())),
        },
    }
}

/// Starts a detached MCP embedding refresh after a model change.
///
/// `sender` is forwarded so the worker thread can push
/// `notifications/resources/updated` once the new embeddings land
/// ([MCP-NOTIFICATIONS]).
pub(super) fn spawn_mcp_embedding_refresh(
    config: SessionBackendConfig,
    state: Arc<Mutex<SessionState>>,
    provider: Arc<dyn EmbeddingProvider>,
    revision: u64,
    sender: Option<NotificationSender>,
) {
    let _join = std::thread::spawn(move || {
        if let Err(error) =
            run_mcp_embedding_refresh(&config, state.as_ref(), provider.as_ref(), revision, &sender)
        {
            warn!(reason = %error, "mcp_embedding_model_refresh_failed");
        }
    });
}

/// Rebuilds the backend session with the selected embedding provider,
/// then pushes report-changed notifications through `sender` (if set).
pub(super) fn run_mcp_embedding_refresh(
    config: &SessionBackendConfig,
    state: &Mutex<SessionState>,
    provider: &dyn EmbeddingProvider,
    revision: u64,
    sender: &Option<NotificationSender>,
) -> Result<(), BackendError> {
    let (session, report) = PipelineSession::initialise(
        config.root.clone(),
        config.min_nodes,
        config.incremental,
        config.config_path.clone(),
        EmbeddingSettings {
            mode: EmbeddingMode::Auto,
            provider: Some(provider),
            batch_yield: live_batch_yield(EmbeddingMode::Auto),
            progress: None,
        },
    )?;
    let generation = {
        let mut guard = super::pipeline::lock_state(state)?;
        if guard.embedding_revision != revision {
            return Ok(());
        }
        guard.session = session;
        guard.report = Arc::new(report);
        guard.generation = guard.generation.saturating_add(1);
        info!(root = %config.root.display(), "mcp_embedding_model_refresh_complete");
        guard.generation
    };
    if let Some(s) = sender {
        push_report_changed(s, generation);
    }
    Ok(())
}
