//! Rendering and corpus-snapshot methods for [`super::PipelineSession`].
//!
//! [`PipelineSession::render`] drives the full LSH → embedding → clustering
//! → ranking → report pipeline over the in-memory corpus.
//! [`PipelineSession::snapshot_corpus_ordered`] flattens the per-file state
//! into the [`super::super::corpus::FingerprintCorpus`] consumed by those
//! stages, in ascending [`crate::state::FileId`] order so the whole pipeline
//! is reproducible ([PIPELINE-DETERMINISM], #301).

use std::collections::HashMap;

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
        let mut clusters = build_ranked_fused_clusters(&corpus.fingerprints, &fused_clusters);
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

    /// Flattens the per-file state into a [`FingerprintCorpus`].
    ///
    /// # QUARANTINED — #301, `[PIPELINE-DETERMINISM]`
    ///
    /// RESTORING THIS OR CALLING THIS FUNCTION IS ILLEGAL.
    /// NO CODE IS ALLOWED TO CALL THIS AND THIS MUST ALWAYS
    /// PANIC AND NOTHING ELSE. POINT CALL SITES ELSEWHERE.
    ///
    /// The deleted body iterated `self.per_file.values()` — a
    /// `HashMap` seeded with `RandomState`, so the fingerprint sequence
    /// was permuted differently on every process:
    ///
    /// ```text
    /// for cached in self.per_file.values() {
    ///     fingerprints.extend(cached.fingerprints.clone());
    ///     trees.push(cached.tree.clone());
    /// }
    /// ```
    ///
    /// Downstream, LSH band buckets pair their minimum-*index* member
    /// with every other member, and the per-pair survival gates
    /// (`LSH_ONLY_MIN_JACCARD`, node floor, fused threshold) then
    /// admit or drop different pairs depending on that random index
    /// assignment. Transitive closure amplifies each flipped pair into
    /// merged-or-not clusters, so two scans of a byte-identical tree
    /// disagreed on cluster ids, `clusters_total`, `duplicated_loc`,
    /// and `duplication_percent` — measured at 1296 vs 1291 clusters,
    /// 30.59% vs 30.08% on the pinned `nest` corpus, and up to a 1.8
    /// point swing on `flutter`. Every run randomly loses real clusters
    /// (false negatives) and every `--fail-over` CI verdict was a
    /// coin flip.
    ///
    /// The accurate replacement is [`Self::snapshot_corpus_ordered`].
    /// Pinned by `crates/deslop/tests/corpus_repos.rs::determinism_gate`.
    ///
    /// # Panics
    ///
    /// Always. This function has no callers and must never gain one.
    #[allow(
        dead_code,
        clippy::panic,
        reason = "[PIPELINE-DETERMINISM] #301 accuracy quarantine. CLAUDE.md mandates \
                  replacing code that causes false negatives with a panic, which the \
                  workspace `panic = \"deny\"` and `-D dead-code` gates would otherwise \
                  reject. The no-suppressions rule yields to the quarantine rule here by \
                  explicit instruction; this allow is legal only on quarantined code."
    )]
    pub(super) fn snapshot_corpus(&self) -> FingerprintCorpus {
        panic!(
            "QUARANTINED #301: snapshot_corpus iterated per_file in HashMap \
             RandomState order, permuting the fingerprint sequence per process and \
             making cluster detection nondeterministic. \
             Use snapshot_corpus_ordered. \
             Pinned by corpus_repos.rs::determinism_gate. \
             per_file_len={}",
            self.per_file.len()
        )
    }

    /// Flattens the per-file state into a [`FingerprintCorpus`] in
    /// ascending [`FileId`] order ([PIPELINE-DETERMINISM], #301).
    ///
    /// [`FileId`]s are issued densely in discovery-walk order, so this
    /// yields the same fingerprint sequence on every run over an
    /// unchanged tree — the property the whole downstream pipeline
    /// (LSH star topology, pair gates, transitive closure) needs to be
    /// reproducible. `per_file` is left empty because the session
    /// already owns the authoritative map — the snapshot is consumed
    /// transiently.
    pub(super) fn snapshot_corpus_ordered(&self) -> FingerprintCorpus {
        let mut file_ids: Vec<FileId> = self.per_file.keys().copied().collect();
        file_ids.sort_unstable();
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
