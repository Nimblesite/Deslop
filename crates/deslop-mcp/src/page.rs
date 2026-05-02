//! Paginated report-page builder shared by `report-get` + `report-query`.
//!
//! Implements [MCP-TOOL-REPORT-PAGINATION] and [MCP-TOOL-REPORT-QUERY]:
//! the live wire returns one [`ReportPage`] per call, each carrying
//! headline metrics plus a slim slice of [`ClusterSummary`] entries
//! (no `members[]`, no full `occurrences[]`). The full record lives
//! behind `cluster-by-id`. Forcing the agent to ask for the deep dive
//! by id is what keeps a single page survivable on a real workspace
//! (where the unsliced report has hit 4 MB+ in production).

use std::path::Path;

use deslop_core::{
    report::{Report, ReportCluster},
    wire_generated::{
        ClusterSummary, OccurrenceSummary, ReportPage, ReportPageFilters, ReportPageInfo,
    },
};

/// Pagination request: zero-based `offset` + cap on returned items.
/// Both come straight from the agent's tool call. The MCP layer
/// rejects missing fields up front, so by the time we see them they
/// are always present.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Zero-based cluster index to start at (within the matched set).
    pub offset: usize,
    /// Maximum number of clusters to return on this page.
    pub limit: usize,
}

/// Builds a [`ReportPage`] over `report` with the matched-and-paged
/// cluster slice. `filters` is the echoed input — pass an empty
/// [`ReportPageFilters`] to opt out of the echoed `filters` field.
#[must_use]
pub fn build_page(
    report: &Report,
    generation: u64,
    pagination: Pagination,
    filters: &ReportPageFilters,
) -> ReportPage {
    let matched: Vec<&ReportCluster> = report
        .clusters
        .iter()
        .filter(|cluster| matches_filters(filters, cluster))
        .collect();
    let total_clusters = matched.len();
    let slice_start = pagination.offset.min(total_clusters);
    let slice_end = slice_start
        .saturating_add(pagination.limit)
        .min(total_clusters);
    let summaries: Vec<ClusterSummary> = matched
        .get(slice_start..slice_end)
        .unwrap_or_default()
        .iter()
        .map(|cluster| cluster_summary_from(cluster))
        .collect();
    let returned = summaries.len();
    ReportPage {
        report_schema_version: report.report_schema_version,
        tool_version: report.tool_version.clone(),
        schema_doc: report.schema_doc.clone(),
        generation,
        files_analysed: report.files_analysed,
        min_nodes: report.min_nodes,
        clusters_hidden: report.clusters_hidden,
        embedding_provenance: report.embedding_provenance.clone(),
        cache_stats: report.cache_stats,
        metrics: report.metrics,
        action_hints: report.action_hints.clone(),
        total_clusters,
        page: ReportPageInfo {
            offset: pagination.offset,
            limit: pagination.limit,
            returned,
        },
        clusters: summaries,
        filters: echo_filters(filters),
    }
}

/// Returns whether `cluster` satisfies every active filter on `filters`.
fn matches_filters(filters: &ReportPageFilters, cluster: &ReportCluster) -> bool {
    if let Some(min) = filters.min_score {
        if cluster.weight < min {
            return false;
        }
    }
    if let Some(min) = filters.min_size {
        if cluster.canonical_node_count < min {
            return false;
        }
    }
    if let Some(bucket) = filters.bucket.as_deref() {
        if cluster.bucket != bucket {
            return false;
        }
    }
    if let Some(language) = filters.language.as_deref() {
        let detected = cluster
            .occurrences
            .first()
            .map_or("unknown", |occ| language_for_path(&occ.path));
        if detected != language {
            return false;
        }
    }
    if let Some(needle) = filters.path_contains.as_deref() {
        let any = cluster.occurrences.iter().any(|occ| {
            occ.path
                .to_str()
                .is_some_and(|path_str| path_str.contains(needle))
        });
        if !any {
            return false;
        }
    }
    true
}

/// Returns a clone of `filters` when at least one knob is set, or
/// `None` so the empty echo block is omitted from the wire entirely.
fn echo_filters(filters: &ReportPageFilters) -> Option<ReportPageFilters> {
    let any_set = filters.language.is_some()
        || filters.bucket.is_some()
        || filters.path_contains.is_some()
        || filters.min_score.is_some()
        || filters.min_size.is_some();
    if any_set {
        Some(filters.clone())
    } else {
        None
    }
}

/// Builds a compact page row from a full report cluster.
fn cluster_summary_from(cluster: &ReportCluster) -> ClusterSummary {
    let first_occurrence = cluster.occurrences.first().map(|occ| OccurrenceSummary {
        path: occ.path.to_string_lossy().into_owned(),
        start_byte: occ.start_byte,
        end_byte: occ.end_byte,
    });
    let language = cluster
        .occurrences
        .first()
        .map_or("unknown", |occ| language_for_path(&occ.path));
    let occurrence_count = cluster.occurrences_total.max(cluster.size);
    ClusterSummary {
        id: cluster.id.clone(),
        bucket: cluster.bucket.clone(),
        score: cluster.weight,
        size_nodes: cluster.canonical_node_count,
        occurrence_count,
        language: language.to_owned(),
        first_occurrence,
    }
}

/// Maps a file extension to a registered language id. Mirrors the
/// renderer's `language_for_path` so MCP summaries stay consistent
/// with the HTML report.
fn language_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("cs") => "csharp",
        Some("rs") => "rust",
        Some("py") => "python",
        _ => "unknown",
    }
}
