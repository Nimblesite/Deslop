//! Report data structures.
//!
//! Implements the agent-first output contract described in
//! [PRINCIPLES-AUDIENCE-AGENT]. JSON is canonical; text and HTML
//! rendering are derived views over the same structs
//! ([OUTPUT-SCHEMA-JSON]). Report hide/exclude semantics follow
//! [EXCLUSION-CONFIG] — hidden occurrences are flagged per-occurrence,
//! hidden-only clusters are dropped and counted in `clusters_hidden`.

use std::{
    collections::HashMap,
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    ast::ByteRange,
    buckets::{bucket_labels, classify_signals, ClusterKind},
    cluster::Cluster,
    config::ExclusionConfig,
    pair::PairScore,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs, RepoMetrics},
    state::{FileId, FileRegistry},
};

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

/// Short playbook entry surfaced at the top of every report so agents
/// can decide how to act before walking the cluster list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHint {
    /// Matches one of the taxonomy rows in the report context
    /// (`type-1-2`, `type-3`, `fused-family`, `lsh-only-weak`).
    pub pattern: String,
    /// One-line recommendation written for an agent reader.
    pub recommendation: String,
}

/// Playbook shown to agents. One entry per bucket in [CLONE-BUCKETS].
/// AI-only surface: uses [`BucketLabels::agent_summary`] so every
/// recommendation carries plain title + action sentence + `Type-N`.
/// Kept aligned with the "Reading the signals together" table in
/// `docs/specs/REPORTING-CONTEXT.md`.
#[must_use]
pub fn default_action_hints() -> Vec<ActionHint> {
    let mut hints = Vec::with_capacity(ClusterKind::all().len());
    for kind in ClusterKind::all() {
        let labels = bucket_labels(kind);
        hints.push(ActionHint {
            pattern: format!("bucket={}", labels.css_suffix),
            recommendation: labels.agent_summary(),
        });
    }
    hints
}

/// A complete analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Stable schema version so agent consumers can parse defensively.
    pub report_schema_version: u32,
    /// Binary / library version that produced the report.
    pub tool_version: String,
    /// Minimum subtree node count used for clustering.
    pub min_nodes: u32,
    /// Number of files analysed.
    pub files_analysed: usize,
    /// Number of clusters hidden from `clusters` because every member
    /// matched a [EXCLUSION-CONFIG] `report_hide` pattern. Makes the
    /// volume of suppressed duplication visible without leaking
    /// contents.
    pub clusters_hidden: usize,
    /// Incremental-cache hit / miss counters for this run
    /// ([PIPELINE-INCREMENTAL]). Defaults to zero when deserialising
    /// older reports that pre-date the field.
    #[serde(default)]
    pub cache_stats: CacheStats,
    /// Repo-wide duplication totals ([METRICS-REPO]). Deserialises as
    /// empty when older (schema v2) reports pre-date the field so
    /// `--from-report` still round-trips them.
    #[serde(default)]
    pub metrics: RepoMetrics,
    /// Markdown schema explanation; see [`SCHEMA_DOC`].
    pub schema_doc: String,
    /// Short agent-oriented playbook; see [`default_action_hints`].
    pub action_hints: Vec<ActionHint>,
    /// Which embedding provider / model / version produced the
    /// `embedding_cos` signals in this report, if any. `None` when
    /// the embedding pass was disabled or failed ([FUSION-EMBED-PROVIDER]).
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Ordered clusters, worst offenders first.
    pub clusters: Vec<ReportCluster>,
}

/// Per-run incremental-cache telemetry. `hits + misses` equals the
/// number of files that reached the parse stage — counters are raw
/// so downstream tooling can compute rates itself. Zero-zero means
/// the pass ran with `incremental: false` (or discovered no files).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Files resolved from the on-disk fingerprint cache.
    pub hits: usize,
    /// Files parsed from scratch because the cache entry was absent,
    /// stale, or unreadable.
    pub misses: usize,
}

/// Provenance block pinning the `(provider_id, model_id, model_version)`
/// triple used when the embedding pass ran. Serialised into the report
/// header per [FUSION-EMBED-PROVIDER] so switching providers/models is
/// visible to consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProvenance {
    /// Registry key of the provider (e.g. `"ollama"`).
    pub provider_id: String,
    /// Human-readable model identifier.
    pub model_id: String,
    /// Opaque model version / digest reported by the provider.
    pub model_version: String,
    /// Embedding dimensionality the provider returned.
    pub dimensions: usize,
}

/// One cluster as it appears in the rendered report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCluster {
    /// Stable short id for cross-referencing.
    pub id: String,
    /// Ranking weight (higher = worse). See [PIPELINE-RANK-WORST-FIRST].
    pub weight: f64,
    /// Size of the cluster (count of cloned occurrences).
    pub size: usize,
    /// AST node count of one canonical member.
    pub canonical_node_count: usize,
    /// Per-cluster signal breakdown so agent consumers can tell **why**
    /// the cluster was flagged ([PRINCIPLES-AUDIENCE-AGENT],
    /// [FUSION-STRATEGY-MAX-SUM]).
    pub signals: ReportSignals,
    /// Canonical bucket ([CLONE-BUCKETS]) the cluster falls into.
    /// One of `"identical" | "nearly_identical" | "loosely_similar" |
    /// "same_behavior"`. Every consumer (renderer, MCP tool, webview)
    /// reads this field instead of re-deriving routing from the signal
    /// triple. `#[serde(default)]` lets `--from-report` keep
    /// round-tripping older reports that pre-date the field; the
    /// renderer re-routes when empty.
    #[serde(default)]
    pub bucket: String,
    /// Every occurrence of the clone.
    pub occurrences: Vec<ReportOccurrence>,
    /// Agent-oriented one-line synthesis (see
    /// [PRINCIPLES-AUDIENCE-AGENT]).
    pub summary: String,
    /// Derived one-line interpretation of the signal combination.
    /// Computed from `signals`; never carries information the signals
    /// don't already convey, but saves the consumer a lookup.
    pub interpretation: String,
}

/// Per-cluster signal breakdown; mirrors
/// [`crate::pair::PairScore`] but kept separate so the report schema is
/// decoupled from the internal struct.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReportSignals {
    /// Mean structural signal across the pairs that formed the cluster.
    pub structural: f64,
    /// Mean token Jaccard estimate across the pairs.
    pub token_jaccard: f64,
    /// Mean embedding cosine similarity across the pairs. 0.0 until
    /// the P5 embedding pass lands.
    pub embedding_cos: f64,
    /// Max-normalized sum of the three components ([0, 3]).
    pub fused: f64,
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

/// A single clone occurrence — a specific `(file, byte_range)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOccurrence {
    /// Absolute path of the source file, relative to the scan root when
    /// possible.
    pub path: PathBuf,
    /// Byte offset of the clone within the file (inclusive).
    pub start_byte: usize,
    /// Byte offset of the end of the clone (exclusive).
    pub end_byte: usize,
    /// True when this occurrence's file matches a [EXCLUSION-CONFIG]
    /// `report_hide` pattern. Hidden occurrences still appear in the
    /// report as long as the cluster has at least one non-hidden
    /// member.
    pub hidden: bool,
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
            );
            let all_hidden = !report_cluster.occurrences.is_empty()
                && report_cluster.occurrences.iter().all(|occ| occ.hidden);
            (report_cluster, all_hidden)
        })
        .collect();
    let clusters_hidden = materialised.iter().filter(|(_, hidden)| *hidden).count();
    let visible_clusters: Vec<ReportCluster> = materialised
        .into_iter()
        .filter_map(|(cluster, hidden)| if hidden { None } else { Some(cluster) })
        .collect();
    let metrics = compute_repo_metrics(&MetricsInputs {
        clusters: inputs.clusters,
        sources: inputs.sources,
        file_languages: inputs.file_languages,
        registry: inputs.registry,
        exclusion: inputs.exclusion,
        analysed_lines: inputs.analysed_lines,
    });
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
        embedding_provenance: inputs.embedding_provenance,
        clusters: visible_clusters,
    }
}

/// Converts one internal [`Cluster`] to a [`ReportCluster`].
fn cluster_to_report<S: BuildHasher>(
    cluster: &Cluster,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
) -> ReportCluster {
    let canonical_node_count = cluster
        .members
        .first()
        .map(|member| member.node_count)
        .unwrap_or_default();
    let occurrences: Vec<ReportOccurrence> = cluster
        .members
        .iter()
        .map(|member| {
            occurrence(
                member.file_id,
                member.byte_range,
                registry,
                file_languages,
                scan_root,
                exclusion,
            )
        })
        .collect();
    let signals: ReportSignals = cluster.signals.into();
    let summary = summarise(
        cluster.members.len(),
        canonical_node_count,
        &occurrences,
        signals,
    );
    let interpretation = interpret(signals, canonical_node_count);
    let bucket = classify_signals(signals).wire_label().to_owned();
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
        bucket,
        occurrences,
        summary,
        interpretation,
    }
}

/// Builds an [`ReportOccurrence`] for a single fingerprint member.
fn occurrence<S: BuildHasher>(
    file_id: FileId,
    byte_range: ByteRange,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
) -> ReportOccurrence {
    let absolute = registry.path(file_id).map(Path::to_path_buf);
    let language = file_languages.get(&file_id).copied().unwrap_or("");
    let hidden = absolute
        .as_deref()
        .is_some_and(|abs| exclusion.is_report_hidden(abs, language));
    let path = absolute.map_or_else(PathBuf::new, |abs| {
        abs.strip_prefix(scan_root)
            .map_or_else(|_| abs.clone(), Path::to_path_buf)
    });
    ReportOccurrence {
        path,
        start_byte: byte_range.start,
        end_byte: byte_range.end,
        hidden,
    }
}

/// Produces a short, agent-readable one-line summary for the cluster.
/// Includes the per-signal breakdown so a downstream agent can tell
/// whether the cluster fired on structure, tokens, or both
/// ([PRINCIPLES-AUDIENCE-AGENT]).
fn summarise(
    size: usize,
    canonical_node_count: usize,
    occurrences: &[ReportOccurrence],
    signals: ReportSignals,
) -> String {
    let locations: Vec<String> = occurrences.iter().take(3).map(format_location).collect();
    let suffix = if occurrences.len() > locations.len() {
        format!(
            " (+{} more)",
            occurrences.len().saturating_sub(locations.len())
        )
    } else {
        String::new()
    };
    format!(
        "{size} copies of a {canonical_node_count}-node subtree at {locs}{suffix} \
         [structural={structural:.2}, token_jaccard={token:.2}, embedding_cos={embed:.2}]",
        locs = locations.join(", "),
        structural = signals.structural,
        token = signals.token_jaccard,
        embed = signals.embedding_cos,
    )
}

/// Maps the signal triple onto a one-line interpretation for AI
/// agents. JSON `cluster.interpretation` is an AI-only surface per
/// [CLONE-BUCKETS-DUAL-LABEL], so the output composes plain title +
/// action sentence + `Type-N`. The `canonical_node_count` is unused
/// today — kept in the signature so callers don't churn; routing
/// lives in `buckets::classify_signals`.
fn interpret(signals: ReportSignals, _canonical_node_count: usize) -> String {
    bucket_labels(classify_signals(signals)).agent_summary()
}

/// Formats one occurrence as `path:start-end`. Extracted so
/// [`summarise`] stays under the 20-line function budget.
fn format_location(occurrence: &ReportOccurrence) -> String {
    format!(
        "{}:{}-{}",
        occurrence.path.display(),
        occurrence.start_byte,
        occurrence.end_byte
    )
}
