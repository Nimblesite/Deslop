//! `LiveApi` trait and concrete [`LiveService`] implementation
//! ([LIVE-QUERY-API]).
//!
//! Exposes the nine query methods documented in `docs/specs/live.md`
//! over an async surface so the LSP / MCP transports can `await`
//! them.

use std::{collections::BTreeMap, path::Path, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    delta::ReportDelta,
    embedding::{EmbeddingProvider, OllamaProvider, StubProvider},
    report::{EmbeddingProvenance, Report, ReportCluster},
};

use super::{
    errors::LiveError,
    session::AnalysisSession,
    wire::{EmbeddingModelInfo, FileReport, FindSimilarRequest, FindSimilarResult, SessionConfig},
};

/// Stable async surface every transport (LSP, MCP) forwards to.
#[async_trait]
pub trait LiveApi: Send + Sync + std::fmt::Debug {
    /// `report/get` — full current snapshot.
    async fn report_get(&self) -> Arc<Report>;

    /// `report/delta` — pull changes since `since`. `None` means the
    /// caller is not behind the head.
    async fn report_delta(&self, since: u64) -> Option<ReportDelta>;

    /// `report/forFile` — clusters whose occurrences touch `path`.
    async fn report_for_file(&self, path: &Path) -> FileReport;

    /// `report/forRange` — clusters overlapping the byte range.
    async fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster>;

    /// `cluster/byId` — fetch cluster by stable id.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches.
    async fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError>;

    /// `duplicates/findSimilar` — agent-facing similarity probe.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError`] for parse failures, unsupported
    /// languages, and out-of-workspace paths.
    async fn find_similar(
        &self,
        request: &FindSimilarRequest,
    ) -> Result<FindSimilarResult, LiveError>;

    /// `embedding/listModels` — enumerate available models.
    async fn embedding_list_models(&self) -> Vec<EmbeddingModelInfo>;

    /// `embedding/setModel` — swap providers atomically.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::ProviderUnreachable`] when the requested
    /// provider cannot be constructed, and other [`LiveError`]
    /// variants when the post-swap pipeline pass fails.
    async fn embedding_set_model(
        &self,
        provider_id: &str,
        model_id: &str,
        endpoint: Option<&str>,
    ) -> Result<Option<EmbeddingProvenance>, LiveError>;

    /// `session/config` — resolved configuration snapshot.
    async fn session_config(&self) -> SessionConfig;
}

/// Concrete [`LiveApi`] implementation backed by an [`AnalysisSession`].
#[derive(Debug)]
pub struct LiveService {
    /// Shared session lock.
    inner: Arc<Mutex<AnalysisSession>>,
    /// History of past report snapshots keyed by generation, used to
    /// answer `report/delta` queries from clients that fell behind.
    previous_reports: Arc<Mutex<BTreeMap<u64, Arc<Report>>>>,
    /// Endpoint passed to `embedding/listModels` lookups. Defaults to
    /// the Ollama default.
    ollama_endpoint: String,
}

impl LiveService {
    /// Constructs a new service wrapping `session`.
    #[must_use]
    pub fn new(session: Arc<Mutex<AnalysisSession>>) -> Self {
        Self {
            inner: session,
            previous_reports: Arc::new(Mutex::new(BTreeMap::new())),
            ollama_endpoint: crate::embedding::DEFAULT_OLLAMA_ENDPOINT.to_owned(),
        }
    }

    /// Overrides the Ollama endpoint used by `embedding/listModels`.
    pub fn set_ollama_endpoint(&mut self, endpoint: String) {
        self.ollama_endpoint = endpoint;
    }

    /// Returns the shared session lock so transports can drive
    /// scheduler-style passes (e.g. on `textDocument/didChange`)
    /// without the service having to know about every transport
    /// affordance.
    #[must_use]
    pub fn session(&self) -> Arc<Mutex<AnalysisSession>> {
        Arc::clone(&self.inner)
    }

    /// Records the latest snapshot so `report/delta` can answer
    /// queries from past generations.
    pub async fn remember_snapshot(&self, generation: u64, report: Arc<Report>) {
        let mut guard = self.previous_reports.lock().await;
        let _previous = guard.insert(generation, report);
    }
}

#[async_trait]
impl LiveApi for LiveService {
    async fn report_get(&self) -> Arc<Report> {
        let guard = self.inner.lock().await;
        guard.report()
    }

    async fn report_delta(&self, since: u64) -> Option<ReportDelta> {
        let (current_gen, current_report) = {
            let guard = self.inner.lock().await;
            (guard.generation(), guard.report())
        };
        if since == current_gen {
            return None;
        }
        let history = self.previous_reports.lock().await;
        let prev = history
            .get(&since)
            .map(|report| (since, report.as_ref().clone()));
        Some(ReportDelta::between(
            prev.as_ref()
                .map(|(generation, report)| (*generation, report)),
            current_gen,
            current_report.as_ref(),
        ))
    }

    async fn report_for_file(&self, path: &Path) -> FileReport {
        let guard = self.inner.lock().await;
        guard.report_for_file(path)
    }

    async fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        let guard = self.inner.lock().await;
        guard.report_for_range(path, start_byte, end_byte)
    }

    async fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        let guard = self.inner.lock().await;
        guard.cluster_by_id(id)
    }

    async fn find_similar(
        &self,
        request: &FindSimilarRequest,
    ) -> Result<FindSimilarResult, LiveError> {
        let guard = self.inner.lock().await;
        guard.find_similar(request)
    }

    async fn embedding_list_models(&self) -> Vec<EmbeddingModelInfo> {
        AnalysisSession::list_embedding_models(&self.ollama_endpoint)
    }

    async fn embedding_set_model(
        &self,
        provider_id: &str,
        model_id: &str,
        endpoint: Option<&str>,
    ) -> Result<Option<EmbeddingProvenance>, LiveError> {
        let provider = build_provider(provider_id, model_id, endpoint)?;
        let mut guard = self.inner.lock().await;
        guard.set_embedding_model(provider)
    }

    async fn session_config(&self) -> SessionConfig {
        let guard = self.inner.lock().await;
        guard.session_config()
    }
}

/// Constructs an [`EmbeddingProvider`] from a `(provider_id, model_id,
/// endpoint?)` tuple.
fn build_provider(
    provider_id: &str,
    model_id: &str,
    endpoint: Option<&str>,
) -> Result<Arc<dyn EmbeddingProvider>, LiveError> {
    match provider_id {
        crate::embedding::STUB_PROVIDER_ID => Ok(Arc::new(StubProvider::new())),
        crate::embedding::DEFAULT_PROVIDER_ID => connect_ollama(model_id, endpoint),
        other => Err(LiveError::UnsupportedProvider {
            requested: other.to_owned(),
            registered: vec![
                crate::embedding::STUB_PROVIDER_ID.to_owned(),
                crate::embedding::DEFAULT_PROVIDER_ID.to_owned(),
            ],
        }),
    }
}

/// Connects to an Ollama provider, lifting transport errors into the
/// live module's error type.
fn connect_ollama(
    model_id: &str,
    endpoint: Option<&str>,
) -> Result<Arc<dyn EmbeddingProvider>, LiveError> {
    let endpoint = endpoint.unwrap_or(crate::embedding::DEFAULT_OLLAMA_ENDPOINT);
    let provider = OllamaProvider::connect(endpoint, model_id).map_err(|err| {
        LiveError::ProviderUnreachable {
            endpoint: endpoint.to_owned(),
            message: err.to_string(),
        }
    })?;
    Ok(Arc::new(provider))
}

/// Convenience constructor that wraps `session` in a [`LiveService`].
/// Free function (not a method) so the call site reads
/// `service_from_session(session)` whether or not the caller already
/// holds a service builder type.
#[must_use]
pub fn service_from_session(session: Arc<Mutex<AnalysisSession>>) -> LiveService {
    LiveService::new(session)
}
