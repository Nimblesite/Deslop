//! Rendering and corpus-snapshot methods for [`super::PipelineSession`].
//!
//! [`PipelineSession::render`] drives the full LSH → embedding → clustering
//! → ranking → report pipeline over the in-memory corpus.
//! [`PipelineSession::snapshot_corpus_ordered`] flattens the per-file state
//! into the [`super::super::corpus::FingerprintCorpus`] consumed by those
//! stages, in ascending workspace-relative-path order so the whole pipeline
//! is reproducible across both reruns and edit history
//! ([PIPELINE-DETERMINISM]).

use std::{collections::HashMap, path::PathBuf};

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    content::attach_content_evidence,
    error::CoreError,
    fingerprint::Fingerprint,
    lsh::band_collisions,
    pair::{candidate_pairs_for_language_policy, cluster_by_transitive_closure},
    report::{render_report, CacheStats, Report, ReportInputs},
    report_metrics::AnalysedLines,
    state::FileId,
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
        let corpus = self.snapshot_corpus_ordered();
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
        // [FUSION-CLUSTER-SIGNALS] One signature space per run: the
        // cross-language space compares any pair when the audit mode is
        // on; the per-language space is exact otherwise. Mixing spaces
        // inside one cluster mean would average incomparable values.
        let measurement_signatures = cross_language_signatures.as_deref().unwrap_or(&signatures);
        let mut clusters = build_ranked_fused_clusters(
            &corpus.fingerprints,
            measurement_signatures,
            &embedding_outcome.vectors,
            &fused_clusters,
        );
        attach_content_evidence(&mut clusters, &corpus.trees, &corpus.sources);
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

    /// Flattens the per-file state into a [`FingerprintCorpus`] in
    /// ascending workspace-relative-path order
    /// ([PIPELINE-DETERMINISM]).
    ///
    /// The sort key must be a property of the workspace *state*, never
    /// of its edit history. [`FileId`]s are append-only: removing and
    /// re-adding a byte-identical file issues a fresh id, so id order
    /// re-shuffles the fingerprint sequence, moves the LSH star centre,
    /// and changes rendered ranges and metrics for identical source. The
    /// normalized path is stable across such churn; the id is only a
    /// tie-breaker so a pathological duplicate registration cannot make
    /// the order ambiguous. `per_file` is left empty because the session
    /// already owns the authoritative map — the snapshot is consumed
    /// transiently.
    pub(super) fn snapshot_corpus_ordered(&self) -> FingerprintCorpus {
        let mut file_ids: Vec<FileId> = self.per_file.keys().copied().collect();
        file_ids.sort_by_cached_key(|id| (self.relative_path_key(*id), *id));
        self.corpus_in_order(&file_ids)
    }

    /// Returns the workspace-relative path for `id`. Falls back to the
    /// registered absolute path when the file sits outside the scan
    /// root, and to an empty path for an unregistered id — every branch
    /// is a function of workspace state, never of registration history.
    fn relative_path_key(&self, id: FileId) -> PathBuf {
        self.registry
            .path(id)
            .map(|path| path.strip_prefix(&self.root).unwrap_or(path).to_path_buf())
            .unwrap_or_default()
    }

    /// Builds the transient corpus snapshot from `file_ids`, preserving
    /// the given order.
    fn corpus_in_order(&self, file_ids: &[FileId]) -> FingerprintCorpus {
        let mut fingerprints: Vec<Fingerprint> = Vec::new();
        let mut trees: Vec<NormalizedNode> = Vec::with_capacity(file_ids.len());
        for cached in file_ids.iter().filter_map(|id| self.per_file.get(id)) {
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
