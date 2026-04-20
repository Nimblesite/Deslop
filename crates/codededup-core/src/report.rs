//! Report data structures.
//!
//! Implements the agent-first output contract described in
//! [PRINCIPLES-AUDIENCE-AGENT]. JSON is canonical; text rendering is a
//! pretty-printer over the same structs.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{ast::ByteRange, cluster::Cluster, pair::PairScore, state::FileRegistry};

/// Current report schema version. Bumped on breaking changes only.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// A complete analysis report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Stable schema version so agent consumers can parse defensively.
    pub report_schema_version: u32,
    /// Binary / library version that produced the report.
    pub tool_version: String,
    /// Minimum subtree node count used for clustering.
    pub min_nodes: u32,
    /// Number of files analysed.
    pub files_analysed: usize,
    /// Ordered clusters, worst offenders first.
    pub clusters: Vec<ReportCluster>,
}

/// One cluster as it appears in the rendered report.
#[derive(Debug, Clone, Serialize)]
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
}

/// Per-cluster signal breakdown; mirrors
/// [`crate::pair::PairScore`] but kept separate so the report schema is
/// decoupled from the internal struct.
#[derive(Debug, Clone, Copy, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct ReportOccurrence {
    /// Absolute path of the source file, relative to the scan root when
    /// possible.
    pub path: PathBuf,
    /// Byte offset of the clone within the file (inclusive).
    pub start_byte: usize,
    /// Byte offset of the end of the clone (exclusive).
    pub end_byte: usize,
}

/// Converts the internal representation into a report ready for
/// serialisation.
#[must_use]
pub fn render_report(
    clusters: &[Cluster],
    registry: &FileRegistry,
    files_analysed: usize,
    min_nodes: u32,
    scan_root: &Path,
) -> Report {
    Report {
        report_schema_version: REPORT_SCHEMA_VERSION,
        tool_version: crate::version().to_owned(),
        min_nodes,
        files_analysed,
        clusters: clusters
            .iter()
            .map(|cluster| cluster_to_report(cluster, registry, scan_root))
            .collect(),
    }
}

/// Converts one internal [`Cluster`] to a [`ReportCluster`].
fn cluster_to_report(
    cluster: &Cluster,
    registry: &FileRegistry,
    scan_root: &Path,
) -> ReportCluster {
    let canonical_node_count = cluster
        .members
        .first()
        .map(|member| member.node_count)
        .unwrap_or_default();
    let occurrences: Vec<ReportOccurrence> = cluster
        .members
        .iter()
        .map(|member| occurrence(member.file_id, member.byte_range, registry, scan_root))
        .collect();
    let signals: ReportSignals = cluster.signals.into();
    let summary = summarise(
        cluster.members.len(),
        canonical_node_count,
        &occurrences,
        signals,
    );
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
        signals,
        occurrences,
        summary,
    }
}

/// Builds an [`ReportOccurrence`] for a single fingerprint member.
fn occurrence(
    file_id: crate::state::FileId,
    byte_range: ByteRange,
    registry: &FileRegistry,
    scan_root: &Path,
) -> ReportOccurrence {
    let path = registry
        .path(file_id)
        .map_or_else(PathBuf::new, |absolute| {
            absolute
                .strip_prefix(scan_root)
                .map_or_else(|_| absolute.to_path_buf(), Path::to_path_buf)
        });
    ReportOccurrence {
        path,
        start_byte: byte_range.start,
        end_byte: byte_range.end,
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
