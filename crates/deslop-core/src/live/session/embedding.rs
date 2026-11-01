//! [LIVE-EMBEDDING-CONSENT] Selected-model embedding refresh on [`AnalysisSession`].

use std::sync::Arc;

use super::{
    super::{
        embedding_refresh::{
            CommittedEmbeddingRefresh, EmbeddingRefreshInput, EmbeddingRefreshJob,
        },
        session_helpers::append_ollama_models,
        wire::EmbeddingModelInfo,
    },
    AnalysisSession,
};
use crate::{
    embedding::{EmbeddingMode, EmbeddingProvider},
    report::Report,
};

impl AnalysisSession {
    /// Queues a selected-model embedding refresh and returns the
    /// detached job description.
    pub(in crate::live) fn prepare_embedding_refresh(
        &mut self,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> EmbeddingRefreshJob {
        self.embedding_provider = provider;
        self.embedding_mode = EmbeddingMode::Auto;
        self.embedding_refresh_revision = self.embedding_refresh_revision.saturating_add(1);
        let total = self
            .pipeline
            .as_ref()
            .map_or(0_u64, |pipeline| pipeline.fingerprint_count() as u64);
        let job = EmbeddingRefreshJob::new(EmbeddingRefreshInput {
            generation: self.generation,
            buffers: self.buffers.clone(),
            revision: self.embedding_refresh_revision,
            root: self.root.clone(),
            min_nodes: self.min_nodes,
            incremental: self.incremental,
            config_path: self.config_path.clone(),
            provider: Arc::clone(&self.embedding_provider),
            total,
            reporter: self.embedding_progress_reporter.clone(),
        });
        job.report_queued();
        job
    }

    /// Whether `job` is still the latest selected-model request.
    ///
    /// Both terminal announcements are gated on this. A superseded job
    /// must stay silent in **either** direction: announcing its
    /// `complete` would publish a verdict for a model the user has
    /// already replaced, and announcing its `failed` would overwrite the
    /// newer job's verdict with a stale failure, since clients carry one
    /// embedding-progress signal rather than one per revision.
    pub(in crate::live) fn embedding_refresh_is_current(&self, job: &EmbeddingRefreshJob) -> bool {
        job.revision == self.embedding_refresh_revision
    }

    /// [LIVE-OPEN-BUFFERS] A detached result must still describe current sources.
    pub(in crate::live) fn embedding_refresh_matches_sources(
        &self,
        job: &EmbeddingRefreshJob,
    ) -> bool {
        job.generation == self.generation && job.buffers == self.buffers
    }

    /// Commits a completed embedding refresh when it is still the
    /// latest selected-model request.
    pub(in crate::live) fn commit_embedding_refresh(
        &mut self,
        job: &EmbeddingRefreshJob,
        report: Report,
    ) -> Option<CommittedEmbeddingRefresh> {
        if !self.embedding_refresh_is_current(job) || !self.embedding_refresh_matches_sources(job) {
            return None;
        }
        let previous_generation = self.generation;
        let previous_report = Arc::clone(&self.latest_report);
        self.generation = self.generation.saturating_add(1);
        let provenance = report.embedding_provenance.clone();
        self.publish_report(Arc::new(report));
        self.freshness
            .record_from_report(&self.root, &self.latest_report);
        Some(CommittedEmbeddingRefresh {
            previous_generation,
            previous_report,
            provenance,
        })
    }

    /// Lists embedding models available to the session.
    ///
    /// Returns models from every registered production provider. When
    /// no production provider is reachable (e.g. Ollama not running),
    /// returns an empty list — the VSIX surfaces the "Ollama not
    /// detected" empty state without a stub fallback.
    #[must_use]
    pub fn list_embedding_models(endpoint: &str) -> Vec<EmbeddingModelInfo> {
        let mut out: Vec<EmbeddingModelInfo> = Vec::new();
        match crate::embedding::list_ollama_models(endpoint) {
            Ok(models) => append_ollama_models(&mut out, models),
            Err(error) => {
                tracing::info!(
                    %error,
                    "ollama_unreachable_no_models_returned",
                );
            }
        }
        out
    }
}
