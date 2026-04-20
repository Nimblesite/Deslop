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
    cluster::Cluster,
    config::ExclusionConfig,
    pair::PairScore,
    state::{FileId, FileRegistry},
};

/// Current report schema version. Bumped on breaking changes only.
pub const REPORT_SCHEMA_VERSION: u32 = 2;

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

/// Playbook shown to agents. Kept short — one entry per row in the
/// "Reading the signals together" table in
/// `docs/specs/REPORTING-CONTEXT.md` so the two never disagree.
#[must_use]
pub fn default_action_hints() -> Vec<ActionHint> {
    vec![
        ActionHint {
            pattern: "structural=1.00, token_jaccard=1.00".to_owned(),
            recommendation:
                "Type-1 or Type-2 exact clone. Safe to extract into a shared function."
                    .to_owned(),
        },
        ActionHint {
            pattern: "structural=1.00, token_jaccard<1.00".to_owned(),
            recommendation:
                "Same AST shape, slightly different tokens. Usually overlapping sibling windows — treat as one clone."
                    .to_owned(),
        },
        ActionHint {
            pattern: "structural=0.00, token_jaccard>=0.90".to_owned(),
            recommendation:
                "Type-3 near-miss. Review both occurrences; differences may be semantically meaningful."
                    .to_owned(),
        },
        ActionHint {
            pattern: "structural=0.00, token_jaccard in [0.70, 0.90)".to_owned(),
            recommendation: "Weak LSH-only signal. Treat as a hint, not a directive.".to_owned(),
        },
        ActionHint {
            pattern: "structural in (0, 1), token_jaccard>=0.95".to_owned(),
            recommendation:
                "Fused cluster spanning several exact-clone bands. Usually genuine duplication across a family of variants."
                    .to_owned(),
        },
    ]
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
    /// Markdown schema explanation; see [`SCHEMA_DOC`].
    pub schema_doc: String,
    /// Short agent-oriented playbook; see [`default_action_hints`].
    pub action_hints: Vec<ActionHint>,
    /// Ordered clusters, worst offenders first.
    pub clusters: Vec<ReportCluster>,
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

/// Converts the internal representation into a report ready for
/// serialisation. Applies [EXCLUSION-CONFIG] `report_hide` semantics:
/// per-occurrence `hidden` flags come from `exclusion`, and any cluster
/// whose every member is hidden is dropped into `clusters_hidden`
/// instead of `clusters`.
#[must_use]
pub fn render_report<S: BuildHasher>(
    clusters: &[Cluster],
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    files_analysed: usize,
    min_nodes: u32,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
) -> Report {
    let materialised: Vec<(ReportCluster, bool)> = clusters
        .iter()
        .map(|cluster| {
            let report_cluster =
                cluster_to_report(cluster, registry, file_languages, scan_root, exclusion);
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
    Report {
        report_schema_version: REPORT_SCHEMA_VERSION,
        tool_version: crate::version().to_owned(),
        min_nodes,
        files_analysed,
        clusters_hidden,
        schema_doc: SCHEMA_DOC.to_owned(),
        action_hints: default_action_hints(),
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
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
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
    let language = file_languages.get(&file_id).copied();
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

/// Maps the signal triple onto a one-line interpretation. Decision
/// table mirrors the "Reading the signals together" table in
/// `docs/specs/REPORTING-CONTEXT.md`; both must change together.
fn interpret(signals: ReportSignals, canonical_node_count: usize) -> String {
    let structural = signals.structural;
    let jaccard = signals.token_jaccard;
    let high_j = jaccard >= 0.95;
    let med_j = jaccard >= 0.90;
    if structural >= 0.99 && jaccard >= 0.99 {
        "Type-1 or Type-2 exact clone. Safe to extract into a shared function.".to_owned()
    } else if structural >= 0.99 {
        "Same AST shape with slight token variation — usually overlapping sibling windows."
            .to_owned()
    } else if structural <= 0.01 && med_j {
        "Type-3 near-miss. Review both occurrences before merging.".to_owned()
    } else if structural > 0.0 && high_j {
        "Fused cluster spanning several exact-clone bands — genuine family of variants.".to_owned()
    } else if canonical_node_count < 40 {
        "Low-information LSH-only match. Treat as a hint, not a directive.".to_owned()
    } else {
        "Weak signal — inspect manually before acting.".to_owned()
    }
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
