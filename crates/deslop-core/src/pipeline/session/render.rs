//! Rendering and corpus-snapshot methods for [`super::PipelineSession`].
//!
//! [`PipelineSession::render`] drives the full LSH → embedding → clustering
//! → ranking → report pipeline over the in-memory corpus.
//! [`PipelineSession::snapshot_corpus`] flattens the per-file state into
//! the [`super::super::corpus::FingerprintCorpus`] consumed by those stages.

use std::collections::HashMap;

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    content::attach_content_agreement,
    error::CoreError,
    fingerprint::Fingerprint,
    lsh::band_collisions,
    pair::{candidate_pairs_for_language_policy, cluster_by_transitive_closure},
    report::{render_report, CacheStats, Report, ReportInputs},
    report_metrics::AnalysedLines,
};

use super::{
    super::{
        config::PipelineConfig, corpus::FingerprintCorpus, embedding_pass::run_embedding_pass,
        signatures::build_cross_language_signatures, signatures::build_signatures_with_languages,
    },
    PipelineSession,
};

impl PipelineSession {
    /// Runs clustering + ranking + rendering over the current
    /// in-memory corpus. Returns a freshly rendered [`Report`].
    pub(super) fn render(
        &mut self,
        config: &PipelineConfig<'_>,
        last_pass_stats: CacheStats,
    ) -> Result<Report, CoreError> {
        let corpus = self.snapshot_corpus();
        tracing::debug!(
            fingerprints = corpus.fingerprints.len(),
            "building signatures"
        );
        let signatures = build_signatures_with_languages(
            &corpus.fingerprints,
            &corpus.trees,
            &self.file_languages,
        );
        tracing::debug!(signatures = signatures.len(), "running LSH band collisions");
        let lsh_pairs = band_collisions(&signatures);
        let cross_language_signatures =
            self.exclusion.allows_cross_language_comparison().then(|| {
                build_cross_language_signatures(
                    &corpus.fingerprints,
                    &corpus.trees,
                    &self.file_languages,
                )
            });
        tracing::debug!(lsh_pairs = lsh_pairs.len(), "running embedding pass");
        let embedding_outcome = run_embedding_pass(config, &corpus)?;
        tracing::debug!(
            embedding_pairs = embedding_outcome.pairs.len(),
            "collecting candidate pairs"
        );
        let pairs = candidate_pairs_for_language_policy(
            &corpus.fingerprints,
            &signatures,
            &lsh_pairs,
            &embedding_outcome.pairs,
            cross_language_signatures.as_deref(),
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        );
        tracing::debug!(
            candidate_pairs = pairs.len(),
            "clustering by transitive closure"
        );
        let fused_clusters = cluster_by_transitive_closure(&pairs);
        tracing::debug!(clusters = fused_clusters.len(), "building ranked clusters");
        let mut clusters = build_ranked_fused_clusters(&corpus.fingerprints, &fused_clusters);
        attach_content_agreement(&mut clusters, &corpus.trees, &corpus.sources);
        tracing::info!(
            ranked_clusters = clusters.len(),
            fingerprints = corpus.fingerprints.len(),
            "render complete"
        );
        Ok(render_report(ReportInputs {
            clusters: &clusters,
            registry: &self.registry,
            file_languages: &self.file_languages,
            files_analysed: self.files_analysed,
            min_nodes: self.min_nodes,
            scan_root: &self.root,
            exclusion: &self.exclusion,
            embedding_provenance: embedding_outcome.provenance,
            cache_stats: last_pass_stats,
            sources: &corpus.sources,
            analysed_lines: &self.analysed_lines,
            boilerplate_ranges: &corpus.boilerplate_ranges,
        }))
    }

    /// Flattens the per-file state into a [`FingerprintCorpus`]
    /// suitable for the downstream LSH / embedding / clustering stages.
    /// `per_file` is left empty because the session already owns the
    /// authoritative map — the snapshot is consumed transiently.
    pub(super) fn snapshot_corpus(&self) -> FingerprintCorpus {
        let mut fingerprints: Vec<Fingerprint> = Vec::new();
        let mut trees: Vec<NormalizedNode> = Vec::with_capacity(self.per_file.len());
        for cached in self.per_file.values() {
            fingerprints.extend(cached.fingerprints.clone());
            trees.push(cached.tree.clone());
        }
        FingerprintCorpus {
            fingerprints,
            trees,
            sources: self.sources.clone(),
            per_file: HashMap::new(),
            cache_stats: CacheStats::default(),
            analysed_lines: AnalysedLines::new(),
            boilerplate_ranges: self.boilerplate_ranges.clone(),
        }
    }
}
