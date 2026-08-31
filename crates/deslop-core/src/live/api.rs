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
    embedding::{EmbeddingProvider, ProviderRegistry, RegistryError},
    report::{EmbeddingProvenance, Report, ReportCluster},
};

use super::{
    embedding_refresh::{run_embedding_refresh, EmbeddingRefreshJob},
    errors::LiveError,
    session::AnalysisSession,
    wire::{EmbeddingModelInfo, FileReport, FindSimilarRequest, FindSimilarResult, SessionConfig},
};

/// Pseudo-provider accepted by `deslop/embeddingSetModel` to disable embeddings.
const OFF_PROVIDER_ID: &str = "off";

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

    /// `merge/plan` — the mechanical call-site merge for a cluster
    /// ([AUTOFIX-MERGE-MCP]). Read-only; refusals arrive as
    /// `ai_or_human` verdicts, never as errors.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches
    /// and [`LiveError::Core`] when the occurrence file fails to parse.
    async fn merge_plan(
        &self,
        cluster_id: &str,
    ) -> Result<crate::wire_generated::MergePlan, LiveError>;

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

    /// Returns every cluster mass in report order.
    async fn all_cluster_masses(&self) -> Vec<u64>;
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

    /// Queues an already-constructed provider for low-priority
    /// embedding refresh work. Returns immediately with `None`; the
    /// new provenance becomes visible through `report_get` after the
    /// background pass commits.
    ///
    /// # Errors
    ///
    /// This path is infallible after provider construction. It stays
    /// fallible to mirror [`LiveApi::embedding_set_model`].
    pub async fn embedding_set_provider(
        &self,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> Result<Option<EmbeddingProvenance>, LiveError> {
        let job = {
            let mut guard = self.inner.lock().await;
            guard.prepare_embedding_refresh(provider)
        };
        self.spawn_embedding_refresh(job);
        Ok(None)
    }

    /// Starts detached low-priority embedding refresh work.
    fn spawn_embedding_refresh(&self, job: EmbeddingRefreshJob) {
        let inner = Arc::clone(&self.inner);
        let previous_reports = Arc::clone(&self.previous_reports);
        let _join = tokio::spawn(async move {
            run_background_refresh(inner, previous_reports, job).await;
        });
    }
}

#[async_trait]
impl LiveApi for LiveService {
    async fn report_get(&self) -> Arc<Report> {
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
        guard.report()
    }

    async fn report_delta(&self, since: u64) -> Option<ReportDelta> {
        let (current_gen, current_report) = {
            let mut guard = self.inner.lock().await;
            guard.refresh_if_stale();
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
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
        guard.report_for_file(path)
    }

    async fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
        guard.report_for_range(path, start_byte, end_byte)
    }

    async fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
        guard.cluster_by_id(id)
    }

    async fn merge_plan(
        &self,
        cluster_id: &str,
    ) -> Result<crate::wire_generated::MergePlan, LiveError> {
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
        crate::live::merge::merge_plan_for(&guard, cluster_id)
    }

    async fn find_similar(
        &self,
        request: &FindSimilarRequest,
    ) -> Result<FindSimilarResult, LiveError> {
        let mut guard = self.inner.lock().await;
        guard.refresh_if_stale();
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
        if provider_id == OFF_PROVIDER_ID {
            return disable_embeddings(self).await;
        }
        let provider = build_provider(provider_id, model_id, endpoint)?;
        self.embedding_set_provider(provider).await
    }

    async fn session_config(&self) -> SessionConfig {
        let guard = self.inner.lock().await;
        guard.session_config()
    }

    async fn all_cluster_masses(&self) -> Vec<u64> {
        let guard = self.inner.lock().await;
        guard
            .report()
            .clusters
            .iter()
            .map(|cluster| cluster.mass)
            .collect()
    }
}

/// Disables the live session embedding pass and records delta history.
///
/// # Errors
///
/// Returns [`LiveError`] when the no-embedding refresh fails.
async fn disable_embeddings(
    service: &LiveService,
) -> Result<Option<EmbeddingProvenance>, LiveError> {
    let (previous_generation, previous_report, to_generation) = {
        let mut guard = service.inner.lock().await;
        let previous_generation = guard.generation();
        let previous_report = guard.report();
        let delta = guard.disable_embeddings()?;
        (previous_generation, previous_report, delta.to_generation)
    };
    service
        .remember_snapshot(previous_generation, previous_report)
        .await;
    tracing::info!(generation = to_generation, "embedding analysis disabled");
    Ok(None)
}

/// Runs one refresh job on a blocking worker and commits its report if
/// the job is still current.
async fn run_background_refresh(
    inner: Arc<Mutex<AnalysisSession>>,
    previous_reports: Arc<Mutex<BTreeMap<u64, Arc<Report>>>>,
    job: EmbeddingRefreshJob,
) {
    let outcome = tokio::task::spawn_blocking(move || run_embedding_refresh(job)).await;
    match outcome {
        Ok(Ok((job, report))) => {
            commit_background_refresh(inner, previous_reports, job, report).await;
        }
        Ok(Err(failure)) => report_background_error(inner, &failure).await,
        Err(error) => tracing::error!(%error, "embedding refresh task failed"),
    }
}

/// Swaps a completed refresh report into the live session and records
/// the previous generation for delta queries.
async fn commit_background_refresh(
    inner: Arc<Mutex<AnalysisSession>>,
    previous_reports: Arc<Mutex<BTreeMap<u64, Arc<Report>>>>,
    job: EmbeddingRefreshJob,
    report: Report,
) {
    let committed = {
        let mut guard = inner.lock().await;
        guard.commit_embedding_refresh(&job, report)
    };
    if let Some(committed) = committed {
        let mut history = previous_reports.lock().await;
        let _previous = history.insert(committed.previous_generation, committed.previous_report);
        drop(history);
        job.report_complete();
        tracing::info!(
            provider = %job.spec.provider_id,
            model = %job.spec.model_id,
            provenance = ?committed.provenance,
            "embedding refresh committed",
        );
    }
}

/// Emits progress and logs for a failed background refresh, when that
/// refresh is still the one the user is waiting on.
///
/// The currency check mirrors the success path's
/// ([`AnalysisSession::commit_embedding_refresh`]): a job the user has
/// already superseded announces nothing. Without it a slow failing job
/// could land its terminal `failed` *after* a newer job announced
/// `complete`, and clients hold one embedding-progress signal rather
/// than one per revision, so the stale failure would be the verdict on
/// screen for a refresh that actually succeeded.
async fn report_background_error(
    inner: Arc<Mutex<AnalysisSession>>,
    failure: &super::embedding_refresh::FailedEmbeddingRefresh,
) {
    let current = {
        let guard = inner.lock().await;
        guard.embedding_refresh_is_current(&failure.job)
    };
    if current {
        failure.job.report_failed(failure.message.clone());
    }
    tracing::warn!(
        error = %failure.message,
        superseded = !current,
        "embedding refresh failed",
    );
}

/// Constructs an [`EmbeddingProvider`] from a `(provider_id, model_id,
/// endpoint?)` tuple via the production [`ProviderRegistry`].
fn build_provider(
    provider_id: &str,
    model_id: &str,
    endpoint: Option<&str>,
) -> Result<Arc<dyn EmbeddingProvider>, LiveError> {
    let registry = ProviderRegistry::production();
    registry
        .build(provider_id, model_id, endpoint)
        .map_err(|err| registry_error_to_live(err, endpoint))
}

/// Lifts a [`RegistryError`] into the live module's error type.
fn registry_error_to_live(error: RegistryError, endpoint: Option<&str>) -> LiveError {
    match error {
        RegistryError::Unsupported {
            requested,
            registered,
        } => LiveError::UnsupportedProvider {
            requested,
            registered,
        },
        RegistryError::Provider(provider_error) => LiveError::ProviderUnreachable {
            endpoint: endpoint
                .unwrap_or(crate::embedding::DEFAULT_OLLAMA_ENDPOINT)
                .to_owned(),
            message: provider_error.to_string(),
        },
    }
}

/// Convenience constructor that wraps `session` in a [`LiveService`].
/// Free function (not a method) so the call site reads
/// `service_from_session(session)` whether or not the caller already
/// holds a service builder type.
#[must_use]
pub fn service_from_session(session: Arc<Mutex<AnalysisSession>>) -> LiveService {
    LiveService::new(session)
}
