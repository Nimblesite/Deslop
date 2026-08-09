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
    buckets::{classify, ClusterKind},
    clone_category::CloneCategory,
    cluster::Cluster,
    cluster_filters::{
        classify_clone_category, is_embedding_role_mismatch, is_noise_pattern,
        is_single_file_declaration_family, ParseCache,
    },
    config::{ExclusionConfig, RankingPolicy},
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
    // Parse each source file at most once for the whole render, shared
    // across every cluster's noise/role checks ([CLONE-NOISE-REPARSE-CACHE]).
    let parse_cache = ParseCache::new();
    let policy = inputs.exclusion.ranking_policy();
    let materialised: Vec<(ReportCluster, bool)> = inputs
        .clusters
        .iter()
        .map(|cluster| materialise_cluster(cluster, &inputs, &parse_cache, policy))
        .collect();
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
        file_languages: inputs.file_languages,
        registry: inputs.registry,
        exclusion: inputs.exclusion,
        analysed_lines: inputs.analysed_lines,
        scan_root: inputs.scan_root,
    });
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
    parse_cache: &ParseCache,
    policy: RankingPolicy,
) -> (ReportCluster, bool) {
    let mut report_cluster = cluster_to_report(
        cluster,
        inputs.registry,
        inputs.file_languages,
        inputs.scan_root,
        inputs.exclusion,
        inputs.sources,
        parse_cache,
    );
    let category = classify_clone_category(
        &cluster.members,
        cluster.literal_fraction,
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
        || cluster_is_hidden(cluster, &report_cluster, inputs, parse_cache);
    (report_cluster, hidden)
}

/// Decides whether a cluster must be dropped from the ranked report.
///
/// The cheap test runs first: a cluster whose every occurrence sits in a
/// report-hidden path (e.g. all members in generated `*.g.dart` /
/// `*.freezed.dart` files) is dropped regardless of the expensive
/// re-parse checks below, so those are skipped. Without this a large
/// generated file is re-walked once per cluster only to be hidden anyway,
/// dominating analysis time on codegen-heavy Dart/Flutter repos
/// ([CLONE-NOISE-REPARSE-CACHE]). The remaining rules:
/// - `#58`: `LooselySimilar` clusters carry only token overlap, no
///   structural/semantic anchor — token-only boilerplate, not duplication.
/// - `#120/#122`: low-structure embedding mega-clusters are report-dominating
///   `Type-4` false positives.
/// - `#69/#70/#71/#72`: re-parsed noise patterns (polymorphic interface
///   impls, test-data variation, REST shape, scaffolding).
/// - `#119`: embedding-dominant `same_behavior` pairs of incompatible roles.
fn cluster_is_hidden<S: BuildHasher>(
    cluster: &Cluster,
    report_cluster: &ReportCluster,
    inputs: &ReportInputs<'_, S>,
    parse_cache: &ParseCache,
) -> bool {
    let occurrences_all_hidden = !report_cluster.occurrences.is_empty()
        && report_cluster.occurrences.iter().all(|occ| occ.hidden);
    if occurrences_all_hidden {
        return true;
    }
    let kind = classify(report_cluster);
    // [DECISION-CROSS-LANGUAGE] Cross-language clusters stay hidden unless the
    // opt-in is enabled — off by default, no heuristics or type-system bridges.
    let token_only_or_mega = (kind == ClusterKind::LooselySimilar
        || is_low_structure_embedding_mega_cluster(report_cluster))
        && !(inputs.exclusion.allows_cross_language_comparison()
            && spans_multiple_languages(&cluster.members, inputs.file_languages));
    let noise = is_noise_pattern(
        &cluster.members,
        inputs.sources,
        inputs.file_languages,
        parse_cache,
    );
    let role_mismatch = kind == ClusterKind::SameBehavior
        && is_embedding_role_mismatch(
            &cluster.members,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        );
    // Issue #197: a single-file `structural_only` family of sibling
    // declarations (REST CRUD / settings / builder methods) is the same
    // evidence-free noise as the cross-file #134 scaffolding. The AST
    // check confines this to declaration families, so worth-extracting
    // statement-window clones stay visible (demoted, not hidden, per
    // [RANK-STRUCTURAL-ONLY]).
    let single_file_declaration_family = kind == ClusterKind::StructuralOnly
        && is_single_file_declaration_family(
            &cluster.members,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        );
    token_only_or_mega || noise || role_mismatch || single_file_declaration_family
}

/// Re-ranks visible clusters by non-hidden occurrence count so mixed
/// clusters dominated by `report_hide` paths cannot push fully-visible
/// clusters down the ranking ([#140 EXCLUSION-CONFIG],
/// [PIPELINE-RANK-WORST-FIRST]). The clone-category multiplier from
/// [RANK-CATEGORY] and the structural-only multiplier from
/// [RANK-STRUCTURAL-ONLY] are folded in here so a `data`-category or
/// shape-only-evidence cluster sinks below comparable full-evidence
/// clones; both multipliers are `1.0` in `keep`/`ignore` modes and for
/// non-matching clusters, which therefore keep their prior weight.
/// Hidden occurrences still travel on each cluster for downstream context.
fn reweigh_by_visible_occurrences(clusters: &mut [ReportCluster], policy: RankingPolicy) {
    for cluster in &mut *clusters {
        let visible = visible_occurrence_count(cluster);
        let base = visible_rank_weight(cluster.canonical_node_count, visible);
        cluster.weight = base
            * category_multiplier(cluster, policy)
            * structural_only_multiplier(cluster, policy);
    }
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// Returns the ranking-weight multiplier for `cluster` under `policy`
/// ([RANK-CATEGORY]). `data`-category clusters get the policy's demote
/// multiplier; everything else stays at `1.0`.
fn category_multiplier(cluster: &ReportCluster, policy: RankingPolicy) -> f64 {
    match CloneCategory::from_wire_label(&cluster.category) {
        CloneCategory::DataTable => policy.data_weight_multiplier(),
        CloneCategory::Logic => 1.0,
    }
}

/// Returns the ranking-weight multiplier for structural-only clusters
/// under `policy` ([RANK-STRUCTURAL-ONLY]). Keyed off the same
/// [`ClusterKind::StructuralOnly`] routing that assigns the wire label,
/// so a labelled cluster is always the cluster the policy demotes
/// (issue #197 inconsistency #1). `data`-category clusters are exempt:
/// their weight belongs to the more specific, user-configurable
/// `[ranking] data_clones` policy ([RANK-CATEGORY]), and stacking both
/// demotions would make `data_clone_weight = 1.0` unable to restore a
/// table that the content gate ([FUSION-CONTENT-GATE]) also routed to
/// the structural-only bucket.
fn structural_only_multiplier(cluster: &ReportCluster, policy: RankingPolicy) -> f64 {
    if classify(cluster) == ClusterKind::StructuralOnly
        && CloneCategory::from_wire_label(&cluster.category) != CloneCategory::DataTable
    {
        policy.structural_only_weight_multiplier()
    } else {
        1.0
    }
}

/// Counts non-hidden occurrences on a rendered cluster. Hidden
/// occurrences still travel with the cluster so consumers retain the
/// "regular code duplicates generated code" context, but ranking
/// ignores them.
fn visible_occurrence_count(cluster: &ReportCluster) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count()
}

/// Mirrors [PIPELINE-RANK-WORST-FIRST] but feeds it the visible
/// occurrence count. Empty visible sets score zero so a cluster that
/// is technically not all-hidden but has only one actionable copy
/// sinks below cleaner clusters with more refactorable duplication.
fn visible_rank_weight(canonical_node_count: usize, visible_size: usize) -> f64 {
    if visible_size < 2 {
        return 0.0;
    }
    let nodes = lossless_u32_to_f64(canonical_node_count.max(1));
    let size_minus_one = lossless_u32_to_f64(visible_size.saturating_sub(1));
    nodes * size_minus_one
}

/// Converts a `usize` to `f64` losslessly. Values past `u32::MAX` are
/// clamped — cluster cardinalities never reach that range in
/// practice but the clamp keeps the math precision-safe under the
/// workspace's `cast_precision_loss` lint.
fn lossless_u32_to_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

/// Returns true for embedding-dominant mega-clusters that are too broad
/// to be actionable. Keeps small Type-4 pairs available while suppressing
/// the real-world "all pytest modules are related" closure failure.
fn is_low_structure_embedding_mega_cluster(cluster: &ReportCluster) -> bool {
    cluster.signals.structural < 0.10
        && cluster.signals.embedding_cos >= 0.80
        && cluster.size > 10
        && cluster.canonical_node_count > 500
}

/// Returns true when a cluster contains more than one parser language id.
fn spans_multiple_languages<S: BuildHasher>(
    members: &[crate::fingerprint::Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let mut languages = members
        .iter()
        .filter_map(|member| file_languages.get(&member.file_id).copied());
    let Some(first) = languages.next() else {
        return false;
    };
    languages.any(|language| language != first)
}

/// Bucket totals emitted for GH#45 classification observability.
#[derive(Default)]
struct BucketDistribution {
    /// Visible identical-code clusters.
    identical: usize,
    /// Visible nearly-identical clusters.
    nearly_identical: usize,
    /// Visible structural-only clusters.
    structural_only: usize,
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
            distribution.add(classify(cluster));
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
            ClusterKind::StructuralOnly => {
                self.structural_only = self.structural_only.saturating_add(1);
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
            structural_only = self.structural_only,
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
