//! Report data structures.
//!
//! Implements the agent-first output contract described in
//! [PRINCIPLES-AUDIENCE-AGENT]. JSON is canonical; text rendering is a
//! pretty-printer over the same structs.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    ast::ByteRange,
    cluster::Cluster,
    state::FileRegistry,
};

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
    /// Every occurrence of the clone.
    pub occurrences: Vec<ReportOccurrence>,
    /// Agent-oriented one-line synthesis (see
    /// [PRINCIPLES-AUDIENCE-AGENT]).
    pub summary: String,
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
    let summary = summarise(cluster.members.len(), canonical_node_count, &occurrences);
    ReportCluster {
        id: cluster.id.clone(),
        weight: cluster.weight,
        size: cluster.members.len(),
        canonical_node_count,
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
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| absolute.to_path_buf())
        });
    ReportOccurrence {
        path,
        start_byte: byte_range.start,
        end_byte: byte_range.end,
    }
}

/// Produces a short, agent-readable one-line summary for the cluster.
fn summarise(size: usize, canonical_node_count: usize, occurrences: &[ReportOccurrence]) -> String {
    let locations: Vec<String> = occurrences
        .iter()
        .take(3)
        .map(|occurrence| {
            format!(
                "{}:{}-{}",
                occurrence.path.display(),
                occurrence.start_byte,
                occurrence.end_byte
            )
        })
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
        "{size} copies of a {canonical_node_count}-node subtree at {locs}{suffix}",
        locs = locations.join(", "),
    )
}
