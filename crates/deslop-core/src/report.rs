//! Report data structures.
//!
//! Implements the agent-first output contract described in
//! [PRINCIPLES-AUDIENCE-AGENT]. JSON is canonical; text and HTML
//! rendering are derived views over the same structs
//! ([OUTPUT-SCHEMA-JSON]). Report hide/exclude semantics follow
//! [EXCLUSION-CONFIG] — hidden occurrences are flagged per-occurrence,
//! hidden-only clusters are dropped and counted in `clusters_hidden`.

use std::{collections::HashMap, hash::BuildHasher, path::Path};

use crate::{
    boilerplate::BoilerplateRange,
    buckets::{classify_signals, ClusterKind},
    cluster::Cluster,
    cluster_filters::is_noise_pattern,
    config::ExclusionConfig,
    pair::PairScore,
    report_boilerplate::build_boilerplate_hints,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs},
    report_render::cluster_to_report,
    state::{FileId, FileRegistry},
};

// `Report`, `CacheStats`, `EmbeddingProvenance`, `ReportCluster`,
// `ReportSignals`, and `ReportOccurrence` are generated from
// `docs/models/live-ipc.td` by `scripts/typediagram-gen.mjs`. The data
// shapes live in `crate::wire_generated`; the impls below stay here.
pub use crate::report_hints::{default_action_hints, ActionHint};
pub use crate::wire_generated::{
    CacheStats, EmbeddingProvenance, Report, ReportCluster, ReportOccurrence, ReportSignals,
};

/// Default occurrence cap applied by [`Report::truncate_for_wire`].
/// Chosen so a pathological 26k-occurrence cluster (real-world alembic
/// migration case) drops from ~2.7 MB to ~10 KB while still giving the
/// agent enough distinct locations to act on. Clients page the rest
/// via `cluster/byId` on the non-live transport.
pub const LIVE_WIRE_OCCURRENCE_CAP: usize = 100;

/// Current report schema version.
///
/// Pinned at `1` for the life of the pre-stable development period.
/// The report shape is still in flux and MAY change between releases
/// while the tool is in its early stages; consumers should treat any
/// pre-1.0 report as best-effort. Once the tool stabilises this will
/// adopt semantic versioning and start bumping on breaking changes.
/// Until then: do **not** bump this constant.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Markdown explaining the report schema. Embedded via `include_str!`
/// from the single source of truth in `docs/specs/REPORTING-CONTEXT.md`
/// so the JSON can never drift from the human-readable description.
pub const SCHEMA_DOC: &str = include_str!("../../../docs/specs/REPORTING-CONTEXT.md");

impl Report {
    /// Projects this report into its live-wire shape: caps every
    /// cluster's `occurrences` at `cap`, blanks the fat derivable
    /// strings (`schema_doc`, `summary`, `interpretation`), and records
    /// the original occurrence count per cluster so clients can surface
    /// "N of M" and page via `cluster/byId`.
    ///
    /// Idempotent: running it twice yields the same shape. Leaves the
    /// CLI / `render_report` path untouched — only transports that ship
    /// reports over a JSON-RPC socket should call this.
    #[must_use]
    pub fn truncate_for_wire(mut self, cap: usize) -> Self {
        self.schema_doc.clear();
        for cluster in &mut self.clusters {
            cluster.occurrences_total = occurrence_count(cluster);
            if cluster.occurrences.len() > cap {
                cluster.occurrences.truncate(cap);
                cluster.occurrences_truncated = true;
            }
            cluster.summary.clear();
            cluster.interpretation.clear();
        }
        self
    }
}

/// Returns the authoritative occurrence count for user-facing copy.
#[must_use]
pub fn occurrence_count(cluster: &ReportCluster) -> usize {
    let total = if cluster.occurrences_total > 0 {
        cluster.occurrences_total
    } else {
        cluster.size
    };
    total.max(cluster.occurrences.len())
}

impl From<PairScore> for ReportSignals {
    fn from(score: PairScore) -> Self {
        Self {
            structural: score.structural,
            token_jaccard: score.token_jaccard,
            embedding_cos: score.embedding_cos,
            fused: score.fused(),
        }
    }
}

/// Parameters accepted by [`render_report`]. Grouped because the
/// list has outgrown the 7-argument function budget without the
/// struct and because adding provenance / languages should not force
/// every call site to re-shuffle positional arguments.
#[derive(Debug)]
pub struct ReportInputs<'a, S: BuildHasher> {
    /// Final ranked clusters from [`crate::cluster`].
    pub clusters: &'a [Cluster],
    /// File registry used to resolve `FileId → path`.
    pub registry: &'a FileRegistry,
    /// `FileId → language_id` map so per-language `report_hide`
    /// patterns apply correctly.
    pub file_languages: &'a HashMap<FileId, &'static str, S>,
    /// Count of files actually parsed (reported in the header).
    pub files_analysed: usize,
    /// Minimum subtree node count used for clustering.
    pub min_nodes: u32,
    /// Absolute scan root for relative-path rendering.
    pub scan_root: &'a Path,
    /// Exclusion config providing `report_hide` semantics.
    pub exclusion: &'a ExclusionConfig,
    /// Embedding provenance — `None` when the embedding pass did not
    /// run or produced no signal.
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Incremental-cache telemetry captured during fingerprinting
    /// ([PIPELINE-INCREMENTAL]).
    pub cache_stats: CacheStats,
    /// Per-file source bytes used to project occurrence `byte_range`s
    /// onto line sets for [METRICS-REPO]. Borrowed; never cloned.
    pub sources: &'a HashMap<FileId, Vec<u8>>,
    /// Per-file analysed-line counts accumulated during the corpus
    /// read-pass ([METRICS-REPO]).
    pub analysed_lines: &'a AnalysedLines,
    /// Import/prologue ranges suppressed before clustering.
    pub boilerplate_ranges: &'a [BoilerplateRange],
}

/// Converts the internal representation into a report ready for
/// serialisation. Applies [EXCLUSION-CONFIG] `report_hide` semantics:
/// per-occurrence `hidden` flags come from `exclusion`, and any cluster
/// whose every member is hidden is dropped into `clusters_hidden`
/// instead of `clusters`.
#[must_use]
pub fn render_report<S: BuildHasher>(inputs: ReportInputs<'_, S>) -> Report {
    let materialised: Vec<(ReportCluster, bool)> = inputs
        .clusters
        .iter()
        .map(|cluster| {
            let report_cluster = cluster_to_report(
                cluster,
                inputs.registry,
                inputs.file_languages,
                inputs.scan_root,
                inputs.exclusion,
                inputs.sources,
            );
            // [#58 FUSION-STRATEGY-GATE-NO-TOKEN-ONLY]: LooselySimilar clusters
            // carry only token-overlap signal with no structural or semantic
            // anchor. Token-only matches (test boilerplate, import scaffolding)
            // push token_jaccard near 1.0 while structural stays near 0,
            // causing these to rank as #1 offenders despite containing no
            // actionable duplication. Exclude them from the human-facing ranked
            // output; the raw analysis data remains available via the pipeline.
            let loosely_similar =
                classify_signals(report_cluster.signals) == ClusterKind::LooselySimilar;
            // Issues #69, #70, #71, #72: re-parse cluster member sources
            // and drop known noise patterns (polymorphic interface
            // implementations, test-data variation, REST endpoint shape,
            // monkeypatch.setenv scaffolding) that survive Type-2
            // normalisation but are not real duplication.
            let noise = is_noise_pattern(&cluster.members, inputs.sources, inputs.file_languages);
            let all_hidden = loosely_similar
                || noise
                || (!report_cluster.occurrences.is_empty()
                    && report_cluster.occurrences.iter().all(|occ| occ.hidden));
            (report_cluster, all_hidden)
        })
        .collect();
    let clusters_hidden = materialised.iter().filter(|(_, hidden)| *hidden).count();
    let visible_clusters: Vec<ReportCluster> = materialised
        .into_iter()
        .filter_map(|(cluster, hidden)| if hidden { None } else { Some(cluster) })
        .collect();
    log_bucket_distribution(&visible_clusters, clusters_hidden);
    let metrics = compute_repo_metrics(&MetricsInputs {
        clusters: inputs.clusters,
        sources: inputs.sources,
        file_languages: inputs.file_languages,
        registry: inputs.registry,
        exclusion: inputs.exclusion,
        analysed_lines: inputs.analysed_lines,
    });
    let boilerplate_hints = build_boilerplate_hints(
        inputs.boilerplate_ranges,
        inputs.registry,
        inputs.scan_root,
        inputs.exclusion,
    );
    Report {
        report_schema_version: REPORT_SCHEMA_VERSION,
        tool_version: crate::version().to_owned(),
        min_nodes: inputs.min_nodes,
        files_analysed: inputs.files_analysed,
        clusters_hidden,
        cache_stats: inputs.cache_stats,
        metrics,
        schema_doc: SCHEMA_DOC.to_owned(),
        action_hints: default_action_hints(),
        boilerplate_hints,
        embedding_provenance: inputs.embedding_provenance,
        clusters: visible_clusters,
    }
}

/// Bucket totals emitted for GH#45 classification observability.
#[derive(Default)]
struct BucketDistribution {
    /// Visible identical-code clusters.
    identical: usize,
    /// Visible nearly-identical clusters.
    nearly_identical: usize,
    /// Visible loosely-similar clusters.
    loosely_similar: usize,
    /// Visible same-behavior clusters.
    same_behavior: usize,
}

impl BucketDistribution {
    /// Counts visible report clusters by canonical bucket.
    fn from_clusters(clusters: &[ReportCluster]) -> Self {
        let mut distribution = Self::default();
        for cluster in clusters {
            distribution.add(classify_signals(cluster.signals));
        }
        distribution
    }

    /// Increments one bucket.
    fn add(&mut self, kind: ClusterKind) {
        match kind {
            ClusterKind::Identical => self.identical = self.identical.saturating_add(1),
            ClusterKind::NearlyIdentical => {
                self.nearly_identical = self.nearly_identical.saturating_add(1);
            }
            ClusterKind::LooselySimilar => {
                self.loosely_similar = self.loosely_similar.saturating_add(1);
            }
            ClusterKind::SameBehavior => self.same_behavior = self.same_behavior.saturating_add(1),
        }
    }

    /// Emits the structured classification distribution.
    fn log(self, visible: usize, hidden: usize) {
        tracing::info!(
            visible,
            hidden,
            identical = self.identical,
            nearly_identical = self.nearly_identical,
            loosely_similar = self.loosely_similar,
            same_behavior = self.same_behavior,
            "bucket distribution",
        );
    }
}

/// Logs the visible cluster bucket distribution after classification.
fn log_bucket_distribution(clusters: &[ReportCluster], hidden: usize) {
    BucketDistribution::from_clusters(clusters).log(clusters.len(), hidden);
}
