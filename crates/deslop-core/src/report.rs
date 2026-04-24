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
    boilerplate::BoilerplateRange,
    buckets::{bucket_labels, classify_signals},
    cluster::Cluster,
    config::ExclusionConfig,
    pair::PairScore,
    report_boilerplate::{build_boilerplate_hints, ReportBoilerplateHint},
    report_location::format_occurrence,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs, RepoMetrics},
    state::{FileId, FileRegistry},
};

pub use crate::report_hints::{default_action_hints, ActionHint};

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
    /// Optional import/prologue hygiene hints from [PIPELINE-BOILERPLATE-FILTER].
    #[serde(default)]
    pub boilerplate_hints: Vec<ReportBoilerplateHint>,
    /// Which embedding provider / model / version produced the
    /// `embedding_cos` signals in this report, if any. `None` when
    /// the embedding pass was disabled or failed ([FUSION-EMBED-PROVIDER]).
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Ordered clusters, worst offenders first.
    pub clusters: Vec<ReportCluster>,
}

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
/// header per [FUSION-EMBED-PROVIDER] so switching providers/models and
/// degraded embedding coverage are visible to consumers.
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
    /// Number of subtree embeddings requested or served from cache.
    #[serde(default)]
    pub attempted_subtrees: usize,
    /// Number of unique successful subtree embeddings fed into ANN.
    #[serde(default)]
    pub indexed_subtrees: usize,
    /// Number of subtree embeddings rejected by the provider. Rejected
    /// subtrees are excluded from the embedding signal, never represented
    /// as zero vectors.
    #[serde(default)]
    pub failed_subtrees: usize,
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
    /// Every occurrence of the clone. On the live wire this vector is
    /// capped by [`Report::truncate_for_wire`]; [`occurrences_total`]
    /// and [`occurrences_truncated`] record the original length so
    /// clients can page via `cluster/byId` if they need the rest.
    pub occurrences: Vec<ReportOccurrence>,
    /// Total number of occurrences before wire truncation. Equals
    /// [`size`] on a full CLI report; set explicitly so live callers
    /// can surface "N of M" without fetching the full cluster.
    /// `#[serde(default)]` lets older reports (pre-cap) round-trip —
    /// callers fall back to `size` when 0.
    #[serde(default)]
    pub occurrences_total: usize,
    /// True when [`occurrences`] was truncated for the wire. False on
    /// CLI reports and on older reports (`#[serde(default)]`).
    #[serde(default)]
    pub occurrences_truncated: bool,
    /// Agent-oriented one-line synthesis (see
    /// [PRINCIPLES-AUDIENCE-AGENT]). Blanked by
    /// [`Report::truncate_for_wire`] because every client re-derives
    /// it from `bucket` + `occurrences` + `signals`.
    pub summary: String,
    /// Derived one-line interpretation; blanked by
    /// [`Report::truncate_for_wire`] because clients re-derive it.
    pub interpretation: String,
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
    /// Unit-bounded fused confidence from the three components.
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

/// Converts one internal [`Cluster`] to a [`ReportCluster`].
fn cluster_to_report<S: BuildHasher>(
    cluster: &Cluster,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
    sources: &HashMap<FileId, Vec<u8>>,
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
        &cluster.members,
        &occurrences,
        sources,
        signals,
    );
    let interpretation = interpret(signals, canonical_node_count);
    let bucket = classify_signals(signals).wire_label().to_owned();
    let occurrences_total = occurrences.len();
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
        bucket,
        occurrences,
        occurrences_total,
        occurrences_truncated: false,
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
    members: &[crate::fingerprint::Fingerprint],
    occurrences: &[ReportOccurrence],
    sources: &HashMap<FileId, Vec<u8>>,
    signals: ReportSignals,
) -> String {
    let locations: Vec<String> = occurrences
        .iter()
        .zip(members)
        .take(3)
        .map(|(occurrence, member)| source_location(occurrence, member.file_id, sources))
        .collect();
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

/// Formats one occurrence through the shared human-location renderer.
fn source_location(
    occurrence: &ReportOccurrence,
    file_id: FileId,
    sources: &HashMap<FileId, Vec<u8>>,
) -> String {
    let source = sources.get(&file_id).map(Vec::as_slice);
    format_occurrence(&occurrence.path, occurrence.start_byte, source)
}
