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
    cluster_filters::{split_noise_verbatim_families, split_structural_families, ParseCache},
    error::CoreError,
    lsh::BandCollisionSource,
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
        // [PERF-FLUTTER-TODO-PAIRS] The LSH pass streams its band
        // collisions straight into the admission-gated candidate
        // construction — no materialised pair vector, no per-pair
        // candidate objects for pairs the survival gate refuses.
        let mut ledger = StageLedger::default();
        // One parse cache for the whole render: the noise split, the
        // ranked build, and the report materialisation all key member
        // analyses by `(file, range)`, so sharing the cache makes each
        // analysis a single computation per run
        // ([PERF-FLUTTER-TODO-CORPUS]).
        let parse_cache = ParseCache::new();
        let stage_started = Instant::now();
        let lsh_source = BandCollisionSource::new(&signatures);
        // [PERF-FLUTTER-TODO-MEMORY] The normalised trees are
        // re-materialised from sources only when a consumer needs them
        // — after the LSH/pair stage whose allocations are the run's
        // other memory peak, so the tree population and the pair
        // population never coincide. The cross-language audit mode
        // reads trees before pairing; the default path defers to the
        // rescue below.
        let (trees, cross_language_signatures, mut pairs, embedding_outcome) =
            self.build_candidate_pairs(config, fingerprints, &signatures, &lsh_source)?;
        ledger.record("candidate_pairs", signatures.len(), pairs.len(), stage_started);
        // Trees for every measurement stage, materialised once, now that
        // the pair-construction allocations are behind us — unless the
        // cross-language audit already materialised them above.
        let trees = match trees {
            Some(already) => already,
            None => self.materialize_trees()?,
        };
        // [FUSION-SHARED-SUBTREE] (gh #408): measure the structural
        // overlap the anchor axis discards before survival drops the
        // enclosing Type-3 pair and leaves only its fragment views.
        let rescue_input = pairs.len();
        let stage_started = Instant::now();
        apply_shared_subtree_rescue(&mut pairs, fingerprints, &trees);
        ledger.record("shared_subtree_rescue", rescue_input, pairs.len(), stage_started);
        let rescue_output = pairs.len();
        let stage_started = Instant::now();
        let fused_clusters = cluster_by_transitive_closure(&pairs);
        ledger.record("transitive_closure", rescue_output, fused_clusters.len(), stage_started);
        // The surviving discovery edges moved into the components, so
        // the flat pair list's last use is behind us — ~600 MB of
        // candidate pairs on a corpus-scale run, freed before the
        // memory-hungry measurement stages ([PERF-FLUTTER-TODO-MEMORY]).
        drop(pairs);
        // [PIPELINE-CLUSTER-ELECT] Transitive closure treats a token
        // band collision like a shared subtree, so one such edge welds
        // two structural families into a component that agrees with
        // itself nowhere, buckets down, and is hidden — losing both
        // families to the presence of each other. Elect the families
        // back out before anything is measured.
        let stage_started = Instant::now();
        let split_input = fused_clusters.len();
        let fused_clusters =
            split_structural_families(fused_clusters, fingerprints, &self.file_languages);
        ledger.record(
            "structural_family_split",
            split_input,
            fused_clusters.len(),
            stage_started,
        );
        // [CLONE-NOISE-VERBATIM-SUBGROUP] Partition a noise family off
        // the byte-identical copy it swept up *before* signals are
        // measured, so the surviving cluster is measured, bucketed and
        // ranked from exactly the occurrences it kept. A component the
        // noise filters do not suppress is handed on untouched.
        let stage_started = Instant::now();
        let noise_input = fused_clusters.len();
        let fused_clusters = split_noise_verbatim_families(
            &fused_clusters,
            fingerprints,
            &self.sources,
            &self.file_languages,
            &parse_cache,
        );
        ledger.record(
            "noise_verbatim_split",
            noise_input,
            fused_clusters.len(),
            stage_started,
        );
        // [FUSION-CLUSTER-SIGNALS] One signature space per run: the
        // cross-language space compares any pair when the audit mode is
        // on; the per-language space is exact otherwise. Mixing spaces
        // inside one cluster mean would average incomparable values.
        let built_alias_space =
            cross_language_signatures.as_deref().map(|space| {
                crate::lsh::SignatureIndex::from_segments([space])
            });
        let measurement_signatures = built_alias_space.as_ref().unwrap_or(&signatures);
        let clusters = self.ranked_clusters(
            fingerprints,
            measurement_signatures,
            &embedding_outcome.vectors,
            &fused_clusters,
            &trees,
            &mut ledger,
        );
        tracing::info!(
            ranked_clusters = clusters.len(),
            fingerprints = fingerprints.len(),
            "render complete"
        );
        ledger.log_summary();
        Ok(render_report(ReportInputs {
            parse_cache: &parse_cache,
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

    /// The LSH/pair construction half of the render: materialises trees
    /// (early only in cross-language audit mode), builds the candidate
    /// pairs.
    fn build_candidate_pairs(
        &self,
        config: &PipelineConfig<'_>,
        fingerprints: &[crate::fingerprint::Fingerprint],
        signatures: &crate::lsh::SignatureIndex<'_>,
        lsh_source: &BandCollisionSource<'_>,
    ) -> Result<
        (
            Option<Vec<crate::ast::NormalizedNode>>,
            Option<Vec<crate::lsh::Signature>>,
            Vec<crate::pair::CandidatePair>,
            crate::pipeline::embedding_pass::EmbeddingOutcome,
        ),
        crate::CoreError,
    > {
        // [PERF-FLUTTER-TODO-MEMORY] Trees are re-materialised from
        // sources only when a consumer needs them — after the LSH/pair
        // stage whose allocations are the run's other memory peak, so
        // the tree population and the pair population never coincide.
        // The cross-language audit mode reads trees before pairing; the
        // default path defers to the rescue in `render`.
        let trees = self
            .exclusion
            .allows_cross_language_comparison()
            .then(|| self.materialize_trees())
            .transpose()?;
        let cross_language_signatures = trees.as_ref().map(|trees| {
            build_cross_language_signatures(fingerprints, trees.as_slice(), &self.file_languages)
        });
        tracing::debug!(signatures = signatures.len(), "streaming LSH band collisions");
        let view = CorpusView {
            fingerprints,
            sources: &self.sources,
        };
        let embedding_outcome = run_embedding_pass(config, &view)?;
        let pairs = candidate_pairs_for_language_policy(
            fingerprints,
            signatures,
            lsh_source,
            &embedding_outcome.pairs,
            cross_language_signatures.as_deref(),
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        );
        Ok((trees, cross_language_signatures, pairs, embedding_outcome))
    }

    /// Re-parses every held source into a normalised tree population,
    /// in parallel, deterministically ordered by file id
    /// ([PERF-FLUTTER-TODO-MEMORY]). The store retains no trees, so the
    /// measurement stages materialise exactly one population at the
    /// moment they need it — after the pair-construction allocations
    /// have peaked — instead of holding gigabytes beside the signature
    /// list for the whole scan. Parsing is a pure function of the held
    /// bytes, so every tree is identical to the one the corpus build
    /// produced and dropped.
    fn materialize_trees(&self) -> Result<Vec<crate::ast::NormalizedNode>, crate::CoreError> {
        let started = Instant::now();
        let mut jobs: Vec<(FileId, &'static str, &[u8])> = self
            .sources
            .iter()
            .filter_map(|(file_id, source)| {
                let language = self.file_languages.get(file_id).copied()?;
                Some((*file_id, language, source.as_slice()))
            })
            .collect();
        jobs.sort_unstable_by_key(|(file_id, _, _)| *file_id);
        let workers = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        // `chunks_mut` demands a non-zero size — an empty corpus has no
        // jobs at all, and even one job must yield a whole shard.
        let shard = jobs.len().div_ceil(workers).max(1);
        let mut slots: Vec<Option<crate::ast::NormalizedNode>> = Vec::with_capacity(jobs.len());
        for _ in 0..jobs.len() {
            slots.push(None);
        }
        let join_result = std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(workers);
            for (slot_base, chunk) in slots.chunks_mut(shard).enumerate() {
                let jobs = &jobs;
                let parsers = &self.parsers;
                let base = slot_base.saturating_mul(shard);
                handles.push(scope.spawn(move || {
                    for (offset, slot) in chunk.iter_mut().enumerate() {
                        let index = base.saturating_add(offset);
                        let Some((file_id, language, source)) = jobs.get(index).copied() else {
                            continue;
                        };
                        let Some(parser) = parsers
                            .iter()
                            .find(|parser| parser.id() == language)
                        else {
                            continue;
                        };
                        *slot = Some(parser.parse_and_normalize(source, file_id)?);
                    }
                    Ok(())
                }));
            }
            let mut outcomes = Vec::with_capacity(handles.len());
            for handle in handles {
                // A panicked parse worker must fail the render, never
                // silently omit its files.
                let joined = handle.join().map_err(|_| crate::CoreError::ParseFailed {
                    language: "unknown",
                });
                outcomes.push(joined);
            }
            outcomes
                .into_iter()
                .collect::<Result<Result<(), crate::CoreError>, crate::CoreError>>()
                .and_then(std::convert::identity)
        });
        join_result?;
        // Every job had a registered parser and parseable source (both
        // proven during the corpus build), so every slot is filled; a
        // `None` would mean a file vanished between stages.
        let mut trees: Vec<crate::ast::NormalizedNode> = Vec::with_capacity(jobs.len());
        for slot in slots {
            trees.push(slot.ok_or(crate::CoreError::ParseFailed {
                language: "unknown",
            })?);
        }
        tracing::info!(
            files = trees.len(),
            elapsed_ms = crate::observe::elapsed_ms(started),
            "normalised trees materialised"
        );
        Ok(trees)
    }

    /// Builds the ranked clusters from the fused ones and records the
    /// `ranked_build` stage row.
    fn ranked_clusters(
        &self,
        fingerprints: &[crate::fingerprint::Fingerprint],
        signatures: &crate::lsh::SignatureIndex<'_>,
        embedding_vectors: &std::collections::HashMap<usize, Vec<f32>>,
        fused_clusters: &[crate::pair::FusedCluster],
        trees: &[crate::ast::NormalizedNode],
        ledger: &mut StageLedger,
    ) -> Vec<crate::cluster::Cluster> {
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
        let started = Instant::now();
        let ranked_input = fused_clusters.len();
        let clusters = build_ranked_fused_clusters(&ClusterBuildInputs {
            fingerprints,
            signatures,
            embedding_vectors,
            fused_clusters,
            trees,
            sources: &self.sources,
            file_languages: &self.file_languages,
            file_paths: &file_paths,
        });
        ledger.record("ranked_build", ranked_input, clusters.len(), started);
        clusters
    }
}

/// One bounded cluster-stage boundary record
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]). A corpus-scale run spends
/// minutes between candidate survival and ranked clusters; at most a
/// handful of these per render, they make the running stage tellable
/// from a hang at the default `info` level, with the elapsed time that
/// attributes the gap.
///
/// The same rows are replayed as one `pipeline stage` event each after
/// the render completes, so a finished run's log reads as a compact
/// per-stage table instead of dense interleaved progress — the summary
/// is the small chunks the full log is broken up into
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]).
#[derive(Default)]
struct StageLedger {
    /// Completed stage boundaries, in run order.
    rows: Vec<StageRow>,
}

/// One recorded stage boundary: name, elapsed time, input and output
/// cardinality.
struct StageRow {
    /// Stage name.
    stage: &'static str,
    /// Wall time spent in the stage, milliseconds.
    elapsed_ms: u64,
    /// Items handed to the stage.
    input: usize,
    /// Items the stage produced.
    output: usize,
}

impl StageLedger {
    /// Records one completed stage boundary.
    fn record(
        &mut self,
        stage: &'static str,
        input: usize,
        output: usize,
        started: Instant,
    ) {
        let elapsed_ms = crate::observe::elapsed_ms(started);
        tracing::info!(
            stage,
            input,
            output,
            elapsed_ms,
            rss_mib = crate::observe::resident_mib(),
            "cluster stage complete"
        );
        self.rows.push(StageRow {
            stage,
            elapsed_ms,
            input,
            output,
        });
    }

    /// Replays every recorded row as one compact event per stage, in
    /// run order, after the render completes.
    fn log_summary(&self) {
        for row in &self.rows {
            tracing::info!(
                stage = row.stage,
                input = row.input,
                output = row.output,
                elapsed_ms = row.elapsed_ms,
                "pipeline stage"
            );
        }
    }
}
