//! Rendering methods for [`super::PipelineSession`].
//!
//! [`PipelineSession::render`] drives the full LSH → embedding →
//! clustering → ranking → report pipeline over the session's canonical
//! corpus store, borrowing every input in place
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]): the flat fingerprint,
//! signature, and tree slices come straight from
//! [`super::store::CorpusStore`], already in workspace-relative-path
//! order ([PIPELINE-DETERMINISM]). A render pass owns no copy of any
//! corpus state — the audited flatten-per-render copy duplicated
//! ~157 MiB of signature bytes alone on the benchmark corpus.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use crate::{
    cluster::{build_ranked_fused_clusters, ClusterBuildInputs},
    cluster_filters::{split_noise_verbatim_families, split_structural_families},
    error::CoreError,
    lsh::band_collisions,
    overlap::apply_shared_subtree_rescue,
    pair::{candidate_pairs_for_language_policy, cluster_by_transitive_closure},
    report::{render_report, CacheStats, Report, ReportInputs},
    state::FileId,
};

use super::{
    super::{
        config::PipelineConfig,
        embedding_pass::{run_embedding_pass, CorpusView},
        signatures::build_cross_language_signatures,
    },
    store::relative_path_key,
    PipelineSession,
};

impl PipelineSession {
    /// Runs clustering + ranking + rendering over the current
    /// in-memory corpus. Returns a freshly rendered [`Report`].
    pub(super) fn render(
        &self,
        config: &PipelineConfig<'_>,
        last_pass_stats: CacheStats,
    ) -> Result<Report, CoreError> {
        // [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] Per-language signatures
        // arrive with the store — built at parse/load time, or attached
        // from the parse store on a cache hit — so the render pass
        // constructs none of them, and borrows rather than copies.
        let fingerprints = self.store.fingerprints();
        let signatures = self.store.signatures();
        tracing::debug!(signatures = signatures.len(), "running LSH band collisions");
        let lsh_pairs = band_collisions(signatures);
        let cross_language_signatures =
            self.exclusion.allows_cross_language_comparison().then(|| {
                build_cross_language_signatures(
                    fingerprints,
                    self.store.trees(),
                    &self.file_languages,
                )
            });
        tracing::debug!(lsh_pairs = lsh_pairs.len(), "running embedding pass");
        let view = CorpusView {
            fingerprints,
            sources: &self.sources,
        };
        let embedding_outcome = run_embedding_pass(config, &view)?;
        tracing::debug!(
            embedding_pairs = embedding_outcome.pairs.len(),
            "collecting candidate pairs"
        );
        let mut pairs = candidate_pairs_for_language_policy(
            fingerprints,
            signatures,
            &lsh_pairs,
            &embedding_outcome.pairs,
            cross_language_signatures.as_deref(),
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        );
        // [FUSION-SHARED-SUBTREE] (gh #408): measure the structural
        // overlap the anchor axis discards before survival drops the
        // enclosing Type-3 pair and leaves only its fragment views.
        apply_shared_subtree_rescue(&mut pairs, fingerprints, self.store.trees());
        let stage_started = Instant::now();
        let fused_clusters = cluster_by_transitive_closure(&pairs);
        log_cluster_stage("transitive_closure", fused_clusters.len(), stage_started);
        // [PIPELINE-CLUSTER-ELECT] Transitive closure treats a token
        // band collision like a shared subtree, so one such edge welds
        // two structural families into a component that agrees with
        // itself nowhere, buckets down, and is hidden — losing both
        // families to the presence of each other. Elect the families
        // back out before anything is measured.
        let stage_started = Instant::now();
        let fused_clusters =
            split_structural_families(fused_clusters, fingerprints, &self.file_languages);
        log_cluster_stage(
            "structural_family_split",
            fused_clusters.len(),
            stage_started,
        );
        // [CLONE-NOISE-VERBATIM-SUBGROUP] Partition a noise family off
        // the byte-identical copy it swept up *before* signals are
        // measured, so the surviving cluster is measured, bucketed and
        // ranked from exactly the occurrences it kept. A component the
        // noise filters do not suppress is handed on untouched.
        let stage_started = Instant::now();
        let fused_clusters = split_noise_verbatim_families(
            fused_clusters,
            fingerprints,
            &self.sources,
            &self.file_languages,
        );
        log_cluster_stage("noise_verbatim_split", fused_clusters.len(), stage_started);
        // [FUSION-CLUSTER-SIGNALS] One signature space per run: the
        // cross-language space compares any pair when the audit mode is
        // on; the per-language space is exact otherwise. Mixing spaces
        // inside one cluster mean would average incomparable values.
        let measurement_signatures = cross_language_signatures.as_deref().unwrap_or(signatures);
        // [PIPELINE-DETERMINISM] (gh #430) Workspace-relative path per
        // fingerprinted file — the second input of the cluster id digest.
        // Built from the fingerprints themselves so every member's file is
        // covered by construction, and keyed on the same
        // workspace-relative form the report renders.
        let file_paths: HashMap<FileId, PathBuf> = fingerprints
            .iter()
            .filter_map(|found| {
                self.registry
                    .path(found.file_id)
                    .map(|path| (found.file_id, relative_path_key(path, &self.root)))
            })
            .collect();
        let clusters = build_ranked_fused_clusters(&ClusterBuildInputs {
            fingerprints,
            signatures: measurement_signatures,
            embedding_vectors: &embedding_outcome.vectors,
            fused_clusters: &fused_clusters,
            trees: self.store.trees(),
            sources: &self.sources,
            file_languages: &self.file_languages,
            file_paths: &file_paths,
        });
        tracing::info!(
            ranked_clusters = clusters.len(),
            fingerprints = fingerprints.len(),
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
            sources: &self.sources,
            analysed_lines: &self.analysed_lines,
            boilerplate_ranges: &self.boilerplate_ranges,
            diff: self.diff_scope.as_ref(),
        }))
    }
}

/// One bounded cluster-stage boundary record
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]). A corpus-scale run spends
/// minutes between candidate survival and ranked clusters; at most a
/// handful of these per render, they make the running stage tellable
/// from a hang at the default `info` level, with the elapsed time that
/// attributes the gap.
fn log_cluster_stage(stage: &'static str, clusters: usize, started: Instant) {
    tracing::info!(
        stage,
        clusters,
        elapsed_ms = crate::observe::elapsed_ms(started),
        "cluster stage complete"
    );
}
