//! Background embedding refresh jobs for live sessions.
//!
//! [LIVE-EMBEDDING-CONSENT] Model selection queues low-priority work
//! without blocking `report/get`; this module owns the detached pass.

use std::{path::PathBuf, sync::Arc};

use crate::{
    embedding::{EmbeddingMode, EmbeddingProvider, EmbeddingSpec},
    pipeline::{EmbeddingSettings, PipelineSession},
    report::{EmbeddingProvenance, Report},
};

use super::{
    errors::LiveError,
    session::EmbeddingProgressReporter,
    session_helpers::{live_batch_yield, report_running_progress},
    wire::{EmbeddingPhase, EmbeddingProgress},
};

/// Immutable description of one queued embedding refresh.
#[derive(Clone)]
pub(super) struct EmbeddingRefreshJob {
    /// Monotonic session-local refresh id.
    pub(super) revision: u64,
    /// Workspace root to rescan outside the session lock.
    pub(super) root: PathBuf,
    /// Active subtree-size floor.
    pub(super) min_nodes: u32,
    /// Whether incremental caches are enabled.
    pub(super) incremental: bool,
    /// Optional config override.
    pub(super) config_path: Option<PathBuf>,
    /// Provider selected by the user.
    pub(super) provider: Arc<dyn EmbeddingProvider>,
    /// Provider identity used for progress payloads.
    pub(super) spec: EmbeddingSpec,
    /// Fingerprint count known when the job was queued.
    pub(super) total: u64,
    /// Optional progress sink supplied by a transport.
    reporter: Option<EmbeddingProgressReporter>,
}

impl std::fmt::Debug for EmbeddingRefreshJob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EmbeddingRefreshJob")
            .field("revision", &self.revision)
            .field("root", &self.root)
            .field("min_nodes", &self.min_nodes)
            .field("incremental", &self.incremental)
            .field("config_path", &self.config_path)
            .field("spec", &self.spec)
            .field("total", &self.total)
            .finish_non_exhaustive()
    }
}

impl EmbeddingRefreshJob {
    /// Builds a job from the session snapshot.
    #[must_use]
    pub(super) fn new(input: EmbeddingRefreshInput) -> Self {
        let spec = input.provider.spec();
        Self {
            revision: input.revision,
            root: input.root,
            min_nodes: input.min_nodes,
            incremental: input.incremental,
            config_path: input.config_path,
            total: input.total,
            provider: input.provider,
            spec,
            reporter: input.reporter,
        }
    }

    /// Emits a queued progress event.
    pub(super) fn report_queued(&self) {
        self.report(EmbeddingPhase::Queued, 0, Some(queue_message()));
    }

    /// Emits a starting progress event.
    fn report_starting(&self) {
        self.report(EmbeddingPhase::Starting, 0, None);
    }

    /// Emits a terminal success event.
    pub(super) fn report_complete(&self) {
        self.report(EmbeddingPhase::Complete, self.total, None);
    }

    /// Emits a terminal failure event.
    pub(super) fn report_failed(&self, message: String) {
        self.report(EmbeddingPhase::Failed, 0, Some(message));
    }

    /// Sends one progress payload when a reporter exists.
    fn report(&self, phase: EmbeddingPhase, done: u64, message: Option<String>) {
        if let Some(reporter) = self.reporter.as_ref() {
            reporter(EmbeddingProgress {
                phase,
                provider_id: self.spec.provider_id.clone(),
                model_id: self.spec.model_id.clone(),
                done,
                total: self.total,
                message,
            });
        }
    }
}

/// Constructor input kept separate so `EmbeddingRefreshJob::new`
/// remains short while preserving explicit field names at the call site.
pub(super) struct EmbeddingRefreshInput {
    /// Monotonic refresh id.
    pub(super) revision: u64,
    /// Workspace root.
    pub(super) root: PathBuf,
    /// Subtree-size floor.
    pub(super) min_nodes: u32,
    /// Incremental-cache flag.
    pub(super) incremental: bool,
    /// Optional config override.
    pub(super) config_path: Option<PathBuf>,
    /// Selected provider.
    pub(super) provider: Arc<dyn EmbeddingProvider>,
    /// Total fingerprints known at queue time.
    pub(super) total: u64,
    /// Optional progress sink.
    pub(super) reporter: Option<EmbeddingProgressReporter>,
}

/// Commit metadata returned when a refresh is still current.
#[derive(Debug)]
pub(super) struct CommittedEmbeddingRefresh {
    /// Generation before the report swap.
    pub(super) previous_generation: u64,
    /// Report snapshot before the swap.
    pub(super) previous_report: Arc<Report>,
    /// Provenance carried by the new report.
    pub(super) provenance: Option<EmbeddingProvenance>,
}

/// Failed background refresh with enough context to notify the client.
#[derive(Debug)]
pub(super) struct FailedEmbeddingRefresh {
    /// Job that failed.
    pub(super) job: Box<EmbeddingRefreshJob>,
    /// User-facing failure message.
    pub(super) message: String,
}

/// Runs the selected-model embedding refresh outside the live-session lock.
pub(super) fn run_embedding_refresh(
    job: EmbeddingRefreshJob,
) -> Result<(EmbeddingRefreshJob, Report), FailedEmbeddingRefresh> {
    job.report_starting();
    let provider_id = job.spec.provider_id.clone();
    let model_id = job.spec.model_id.clone();
    let total = job.total;
    let progress = |done: usize| {
        report_running_progress(job.reporter.as_ref(), &provider_id, &model_id, done, total);
    };
    let embedding = EmbeddingSettings {
        mode: EmbeddingMode::Auto,
        provider: Some(job.provider.as_ref()),
        batch_yield: live_batch_yield(EmbeddingMode::Auto),
        progress: Some(&progress),
    };
    let outcome = PipelineSession::initialise(
        job.root.clone(),
        job.min_nodes,
        job.incremental,
        job.config_path.clone(),
        embedding,
    );
    match outcome {
        Ok((_, report)) => admit_refresh_report(job, report),
        Err(error) => Err(FailedEmbeddingRefresh {
            job: Box::new(job),
            message: LiveError::from(error).to_string(),
        }),
    }
}

/// Admits a refreshed report only when the pass produced embeddings.
///
/// [LIVE-EMBEDDING-CONSENT] terminal-phase rule (GH #370): the terminal
/// phase is a function of the embeddings the pass produced, never of
/// whether the pipeline returned a report. A refresh that produced none
/// is a typed failure — it must terminate `failed` with `done = 0` and
/// leave the last good report untouched, not commit an embeddings-off
/// snapshot announced as `complete`.
///
/// Two shapes of "produced none" reach here, and both are failures:
///
/// - **The provider rejected everything.** Provenance records
///   `indexed = 0` against a non-zero `failed` — see [`rejection_message`].
/// - **The pass never ran.** A refresh always carries an explicitly
///   selected provider and runs under [`EmbeddingMode::Auto`], and
///   `pipeline::run_embedding_pass` deliberately swallows a provider
///   error in that mode ("continuing without Type-4 recall"), returning
///   a report with **no** provenance at all. That is the right call for
///   an automatic pass and the wrong one for a model the user chose: a
///   provider that answered the selection probe and was gone by the time
///   the refresh ran would otherwise commit an embeddings-off snapshot
///   announced as `complete`. Absent provenance is therefore absent
///   embeddings, never a silent success.
///
/// Pinned from both directions by
/// `deslop-lsp/tests/embedding_failure_progress.rs`:
/// `rejected_embedding_refresh_reports_failure_and_preserves_last_good_report`
/// and `vanished_provider_refresh_reports_failure_and_preserves_last_good_report`.
fn admit_refresh_report(
    job: EmbeddingRefreshJob,
    report: Report,
) -> Result<(EmbeddingRefreshJob, Report), FailedEmbeddingRefresh> {
    match no_embeddings_message(report.embedding_provenance.as_ref(), &job.spec) {
        None => Ok((job, report)),
        Some(message) => Err(FailedEmbeddingRefresh {
            job: Box::new(job),
            message,
        }),
    }
}

/// Returns the user-facing failure message when a refresh produced no
/// embeddings, or `None` when it indexed at least one subtree.
fn no_embeddings_message(
    provenance: Option<&EmbeddingProvenance>,
    spec: &EmbeddingSpec,
) -> Option<String> {
    let Some(provenance) = provenance else {
        return Some(format!(
            "Embedding refresh produced no embeddings: the {provider}/{model} pass \
             did not run — the provider was unreachable or its model unavailable. \
             The previous report is unchanged.",
            provider = spec.provider_id,
            model = spec.model_id,
        ));
    };
    rejection_message(provenance)
}

/// Returns the user-facing failure message when the provider rejected
/// every attempted subtree, or `None` when the pass indexed anything.
fn rejection_message(provenance: &EmbeddingProvenance) -> Option<String> {
    if provenance.indexed_subtrees > 0 || provenance.failed_subtrees == 0 {
        return None;
    }
    Some(format!(
        "Embedding refresh produced no embeddings: {provider}/{model} rejected \
         all {failed} of {attempted} attempted subtrees. The previous report \
         is unchanged.",
        provider = provenance.provider_id,
        model = provenance.model_id,
        failed = provenance.failed_subtrees,
        attempted = provenance.attempted_subtrees,
    ))
}

/// Returns the queued progress message shown to clients.
fn queue_message() -> String {
    "Queued as low-priority background embedding work.".to_owned()
}
