//! Report data structures.
//!
//! Implements the agent-first output contract described in
//! [PRINCIPLES-AUDIENCE-AGENT]. JSON is canonical; text and HTML
//! rendering are derived views over the same structs
//! ([OUTPUT-SCHEMA-JSON]). Report hide/exclude semantics follow
//! [EXCLUSION-CONFIG] — hidden occurrences are flagged per-occurrence,
//! hidden-only clusters are dropped and counted in `clusters_hidden`.

use std::{collections::HashMap, hash::BuildHasher, path::Path};

/// Cluster visibility rules and their trace.
mod hidden;
use hidden::{cluster_is_hidden, log_hidden_cluster, BucketDistribution};

use crate::{
    boilerplate::BoilerplateRange,
    buckets::{classify, ClusterKind},
    clone_category::CloneCategory,
    cluster::Cluster,
    cluster_filters::{classify_clone_category, ParseCache},
    config::{ExclusionConfig, RankingPolicy},
    pair::PairScore,
    report_boilerplate::build_boilerplate_hints,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs},
    report_render::{cluster_to_report, ReportSources},
    report_weight::reweigh_by_visible_occurrences,
    state::{FileId, FileRegistry},
};

// `Report`, `CacheStats`, `EmbeddingProvenance`, `ReportCluster`,
// `ReportSignals`, and `ReportOccurrence` are generated from
// `docs/models/live-ipc.td` by `scripts/typediagram/generate.mjs`. The data
// shapes live in `crate::wire_generated`; the impls below stay here.
pub use crate::report_hints::{default_action_hints, ActionHint};
pub use crate::wire_generated::{
    CacheStats, EmbeddingProvenance, Report, ReportCluster, ReportOccurrence, ReportSignalSource,
    ReportSignals,
};

/// Serde default for [`ReportSignals::agreement`] when replaying a
/// report written before the content gate existed: an absent field
/// means nothing was measured, and the unmeasured convention is full
/// agreement so a missing measurement never demotes a cluster the
/// original run vouched for ([FUSED-CONTENT-GATE],
/// [`crate::content::ContentEvidence::unmeasured`]).
#[must_use]
pub fn unmeasured_agreement() -> f64 {
    1.0
}

/// Default occurrence cap applied by [`Report::truncate_for_wire`].
/// Chosen so a pathological 26k-occurrence cluster (real-world alembic
/// migration case) drops from ~2.7 MB to ~10 KB while still giving the
/// agent enough distinct locations to act on. Clients page the rest
/// via `cluster/byId` on the non-live transport.
pub const LIVE_WIRE_OCCURRENCE_CAP: usize = 100;

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
    /// `evidence_verdict` deliberately survives: it is engine-authored
    /// and not client-derivable, so blanking it would force clients to
    /// grow their own verdict engine — the exact duplicate-calculation
    /// defect the field exists to remove.
    ///
    /// Idempotent: running it twice yields the same shape. Leaves the
    /// CLI / `render_report` path untouched — only transports that ship
    /// reports over a JSON-RPC socket should call this.
    #[must_use]
    pub fn truncate_for_wire(mut self, cap: usize) -> Self {
        self.schema_doc.clear();
        for cluster in &mut self.clusters {
            let count = occurrence_count(cluster);
            cluster.occurrences_total = count;
            cluster.occurrence_count = count;
            if cluster.occurrences.len() > cap {
                cluster.occurrences.truncate(cap);
                cluster.occurrences_truncated = true;
            }
            // [FUSED-CLUSTER-SIGNALS] The named signal source must stay
            // inside the occurrences the wire actually carries; a
            // truncated source index would dangle.
            cluster.signal_source = cluster.signal_source.filter(|source| {
                source.left < cluster.occurrences.len() && source.right < cluster.occurrences.len()
            });
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

/// Distinct paths among a cluster's visible (non-hidden) occurrences —
/// the cross-file screen every consolidation surface shares
/// ([AUTOFIX-CONSOLIDATE-SURFACE]). Two or more distinct paths imply
/// two or more visible occurrences, so callers need no separate count
/// check.
#[must_use]
pub fn distinct_visible_path_count(cluster: &ReportCluster) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .map(|occurrence| &occurrence.path)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

impl From<PairScore> for ReportSignals {
    /// The content triple is left at zero here and stamped later by
    /// [`crate::buckets::content_gated_signals`] ([FUSED-CONTENT-GATE],
    /// #344). A `PairScore` is the deterministic pair evidence, produced
    /// before any content is measured, so it has nothing truthful to put
    /// in those fields — every rendered cluster passes through the gate,
    /// which does.
    fn from(score: PairScore) -> Self {
        let mut signals = Self {
            structural: score.structural,
            token_jaccard: score.token_jaccard,
            // Stamped below through the one [`ReportSignals::shape_score`]
            // definition once the source fields exist.
            shape: 0.0,
            embedding_cos: score.embedding_cos,
            fused: score.bounded_fused(),
            agreement: 0.0,
            rename_consistency: 0.0,
            literal_fraction: 0.0,
        };
        signals.shape = signals.shape_score();
        signals
    }
}

impl ReportSignals {
    /// The shape reading — the stronger of `structural` and
    /// `token_jaccard`, two views of one normalised representation, so
    /// the max is what "the shape matched" means
    /// ([FUSED-CONTENT-GATE]). The single definition behind the wire
    /// `shape` field, the content gate's fused reduction, and the
    /// evidence verdict; consumers render the stamped field verbatim
    /// and never re-derive the max.
    #[must_use]
    pub fn shape_score(&self) -> f64 {
        self.structural.max(self.token_jaccard)
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
    /// Verified diff scope when the run carried `--diff`
    /// ([CLI-ARG-DIFF]). Drives occurrence/cluster tagging
    /// ([OUTPUT-SCHEMA-DIFF-TAGS]) and `metrics.diff`
    /// ([METRICS-DIFF-SCOPE]); `None` leaves every diff field absent.
    pub diff: Option<&'a crate::diff_scope::DiffScope>,
    /// The run-shared parse cache ([PERF-FLUTTER-TODO-CORPUS]): the
    /// same `(file, range)` member analyses the noise split already
    /// computed, reused here for the hidden/category/role checks so a
    /// corpus-scale render never recomputes them.
    pub parse_cache: &'a crate::cluster_filters::ParseCache,
}

/// Stage label on the cluster-noise counters emitted once the render
/// pass has finished convicting ([PERF-FLUTTER-TODO-OBSERVABILITY]).
///
/// The counters live on the run-shared [`ParseCache`], so by this point
/// they hold the noise-split stage *plus* every render-stage check —
/// hence `run_cumulative`, not a stage name. The distinction is not
/// cosmetic: the split stage emitted `fired=0` over a 14-member
/// component and that partial total was twice read as the whole run's,
/// concluding the filters never examined a cluster they had in fact
/// examined and declined (gh #434). Keying the counters per stage, so
/// each record reports its own numbers instead of a running sum, needs
/// `cluster_filters/` and is tracked in gh #478.
const NOISE_TOTALS_RUN_STAGE: &str = "run_cumulative_after_report_render";

/// Converts the internal representation into a report ready for
/// serialisation. Applies [EXCLUSION-CONFIG] `report_hide` semantics:
/// per-occurrence `hidden` flags come from `exclusion`, and any cluster
/// whose every member is hidden is dropped into `clusters_hidden`
/// instead of `clusters`.
#[must_use]
/// # Panics
///
/// Only on an internal invariant: every cluster handed in must
/// materialise into its slot (`order` covers exactly the input range,
/// so a panic here means a cluster vanished mid-render — a defect, not
/// an input condition).
pub fn render_report<S: BuildHasher>(inputs: ReportInputs<'_, S>) -> Report {
    // The cache arrives from the pipeline: one parse per file and one
    // member analysis per `(file, range)` for the whole run
    // ([CLONE-NOISE-REPARSE-CACHE], [PERF-FLUTTER-TODO-CORPUS]).
    let parse_cache = inputs.parse_cache;
    let report_sources = ReportSources::new(inputs.sources);
    let policy = inputs.exclusion.ranking_policy();
    // [PERF-FLUTTER-TODO-MEMORY] Materialise in minimum-member-file
    // order so each file's clusters arrive together and the bounded
    // [`ParseCache`](crate::cluster_filters::ParseCache) tree LRU stays
    // hot; results land at their input position, so the report is
    // byte-identical to a straight in-order map.
    let mut order: Vec<usize> = (0..inputs.clusters.len()).collect();
    order.sort_by_key(|&index| {
        inputs
            .clusters
            .get(index)
            .and_then(|cluster| cluster.members.iter().map(|member| member.file_id).min())
    });
    let mut slots: Vec<Option<(ReportCluster, bool)>> =
        (0..inputs.clusters.len()).map(|_| None).collect();
    for index in order {
        let Some(cluster) = inputs.clusters.get(index) else {
            continue;
        };
        let built = materialise_cluster(cluster, &inputs, &report_sources, parse_cache, policy);
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(built);
        }
    }
    // Every render-stage noise check has now run: `cluster_is_hidden` is
    // the only caller downstream of the split stage, and it is reached
    // solely from the loop above. Without this the render stage's
    // convictions were recorded into the shared counters and discarded
    // unread ([PERF-FLUTTER-TODO-OBSERVABILITY]).
    parse_cache.log_noise_totals(NOISE_TOTALS_RUN_STAGE);
    let materialised: Vec<(ReportCluster, bool)> = slots.into_iter().flatten().collect();
    // The order covered exactly `0..len`, so every slot is filled; a
    // short collect would mean a cluster vanished mid-render.
    assert_eq!(
        materialised.len(),
        inputs.clusters.len(),
        "every cluster must materialise into its slot"
    );
    let clusters_hidden = materialised.iter().filter(|(_, hidden)| *hidden).count();
    // The metric must count the same clusters the report renders, so it
    // sees only the survivors of `materialise_cluster` — never a cluster
    // dropped as report-hidden, noise, or structural-only sibling
    // boilerplate. Aligned positionally with `inputs.clusters` because
    // `materialised` was mapped over it in order ([METRICS-REPO]).
    let visible_internal: Vec<&Cluster> = inputs
        .clusters
        .iter()
        .zip(&materialised)
        .filter_map(|(cluster, (_, hidden))| (!hidden).then_some(cluster))
        .collect();
    let mut visible_clusters: Vec<ReportCluster> = materialised
        .into_iter()
        .filter_map(|(cluster, hidden)| if hidden { None } else { Some(cluster) })
        .collect();
    reweigh_by_visible_occurrences(&mut visible_clusters, policy);
    log_bucket_distribution(&visible_clusters, clusters_hidden);
    let mut metrics = compute_repo_metrics(&MetricsInputs {
        clusters: &visible_internal,
        sources: inputs.sources,
        line_indices: report_sources.line_indices(),
        file_languages: inputs.file_languages,
        registry: inputs.registry,
        exclusion: inputs.exclusion,
        analysed_lines: inputs.analysed_lines,
        scan_root: inputs.scan_root,
        diff: inputs.diff,
    });
    // [OUTPUT-SCHEMA-DIFF-TAGS] Tags are stamped on the exact cluster
    // list the report carries — after hiding and reweighing — so a
    // tagged report and an untagged one always list identical clusters.
    if let Some(scope) = inputs.diff {
        crate::diff_scope::tag_clusters(&mut visible_clusters, scope);
    }
    // Resolve the [EXIT-CODES] duplication gate here so every surface that
    // renders through this path carries the breach verdict — the live
    // LSP/MCP servers, not just the CLI. `compute_repo_metrics` leaves it
    // `none()` because it has no config; the CLI may still override the
    // result via `--fail-over` / `--no-fail-over` after this returns.
    let measured = metrics.duplication_percent;
    metrics.threshold = inputs.exclusion.resolve_threshold(measured);
    let boilerplate_hints = build_boilerplate_hints(
        inputs.boilerplate_ranges,
        inputs.registry,
        inputs.scan_root,
        inputs.exclusion,
    );
    Report {
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
        // Set by `diff_scope::apply_only_changed` when the CLI filters;
        // absent otherwise ([CLI-ARG-ONLY-CHANGED]).
        clusters_outside_diff: None,
    }
}

/// Builds one [`ReportCluster`], stamps its [`CloneCategory`]
/// ([RANK-CATEGORY]), and decides whether it is hidden from the ranked
/// report. A cluster is hidden when it is a known noise pattern *or* when it
/// is a `data`-category cluster and the policy is `ignore`. The category is
/// classified once here using the shared parse cache and the result is both
/// stamped onto the wire cluster and reused for the drop decision, so the
/// re-parse never happens twice.
fn materialise_cluster<S: BuildHasher>(
    cluster: &Cluster,
    inputs: &ReportInputs<'_, S>,
    report_sources: &ReportSources<'_>,
    parse_cache: &ParseCache,
    policy: RankingPolicy,
) -> (ReportCluster, bool) {
    let mut report_cluster = cluster_to_report(
        cluster,
        inputs.registry,
        inputs.file_languages,
        inputs.scan_root,
        inputs.exclusion,
        report_sources,
        parse_cache,
    );
    let category = classify_clone_category(
        &cluster.members,
        cluster.content.literal_fraction,
        inputs.sources,
        inputs.file_languages,
        parse_cache,
    );
    category
        .wire_label()
        .clone_into(&mut report_cluster.category);
    let dropped_as_data = category == CloneCategory::DataTable && policy.drops_data_clusters();
    // [RANK-STRUCTURAL-ONLY] `ignore` drops shape-only-evidence
    // clusters the same way the data `ignore` policy drops tables.
    let dropped_as_structural_only =
        policy.drops_structural_only() && classify(&report_cluster) == ClusterKind::StructuralOnly;
    let hidden = dropped_as_data
        || dropped_as_structural_only
        || cluster_is_hidden(cluster, &report_cluster, inputs, parse_cache, category);
    if hidden {
        log_hidden_cluster(
            &report_cluster,
            cluster.content,
            dropped_as_data,
            dropped_as_structural_only,
        );
    }
    (report_cluster, hidden)
}

/// Logs the visible cluster bucket distribution after classification.
fn log_bucket_distribution(clusters: &[ReportCluster], hidden: usize) {
    BucketDistribution::from_clusters(clusters).log(clusters.len(), hidden);
}
