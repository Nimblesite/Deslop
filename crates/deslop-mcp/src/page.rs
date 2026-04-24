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

use deslop_core::report::{Report, ReportCluster};
use serde::Serialize;
use serde_json::{json, Value};

/// Slim, agent-sized projection of a [`ReportCluster`]. Drops the full
/// `members` + `occurrences` arrays that the canonical report carries —
/// those live behind `cluster-by-id`. Carries a single representative
/// occurrence so the agent can navigate to the cluster without a second
/// round-trip.
#[derive(Debug, Clone, Serialize)]
pub struct ClusterSummary {
    /// Stable 16-char id; pass to `cluster-by-id` for the full record.
    pub id: String,
    /// Canonical bucket label ([CLONE-BUCKETS]).
    pub bucket: String,
    /// Worst-first ranking score; mirrors `ReportCluster.weight`.
    pub score: f64,
    /// Representative subtree node count (`canonical_node_count`).
    pub size_nodes: usize,
    /// Total occurrences across the cluster, taken from
    /// `occurrences_total` so wire-truncated counts still surface
    /// honestly.
    pub occurrence_count: usize,
    /// Detected source language for the first occurrence (`csharp`,
    /// `rust`, `python`, …) or `"unknown"` when the extension is not
    /// registered.
    pub language: &'static str,
    /// One representative occurrence so the agent can navigate without
    /// fetching the full cluster. `None` when the cluster has no
    /// occurrences (defensive — the renderer never produces such
    /// clusters today).
    pub first_occurrence: Option<OccurrenceSummary>,
}

/// Single representative occurrence on a [`ClusterSummary`]. Bytes are
/// the native unit on `ReportOccurrence`; agents convert to lines on
/// demand via the file contents.
#[derive(Debug, Clone, Serialize)]
pub struct OccurrenceSummary {
    /// Workspace-relative path of the occurrence.
    pub path: String,
    /// Inclusive byte offset of the clone within the file.
    pub start_byte: usize,
    /// Exclusive byte offset of the end of the clone.
    pub end_byte: usize,
}

/// Filter knobs accepted by `report-query`. All combine with logical
/// `AND`; absent fields are treated as "match everything". Filtering
/// happens before pagination so `total_clusters` reflects the matched
/// set, not the unfiltered universe.
#[derive(Debug, Default, Clone)]
pub struct QueryFilters {
    /// Match clusters whose detected language equals this id.
    pub language: Option<String>,
    /// Match clusters whose canonical bucket equals this id (e.g.
    /// `"identical"`, `"nearly_identical"`).
    pub bucket: Option<String>,
    /// Match clusters where any occurrence path contains this
    /// case-sensitive substring.
    pub path_contains: Option<String>,
    /// Match clusters whose `weight` is `>= min_score`.
    pub min_score: Option<f64>,
    /// Match clusters whose `canonical_node_count` is `>= min_size`.
    pub min_size: Option<usize>,
}

impl QueryFilters {
    /// Returns whether `cluster` satisfies every active filter.
    fn matches(&self, cluster: &ReportCluster) -> bool {
        if let Some(min) = self.min_score {
            if cluster.weight < min {
                return false;
            }
        }
        if let Some(min) = self.min_size {
            if cluster.canonical_node_count < min {
                return false;
            }
        }
        if let Some(bucket) = self.bucket.as_deref() {
            if cluster.bucket != bucket {
                return false;
            }
        }
        if let Some(language) = self.language.as_deref() {
            let detected = cluster
                .occurrences
                .first()
                .map_or("unknown", |occ| language_for_path(&occ.path));
            if detected != language {
                return false;
            }
        }
        if let Some(needle) = self.path_contains.as_deref() {
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

    /// Renders the active filters as a JSON object so transcripts can
    /// reproduce the call. Absent filters are omitted (not nulled) to
    /// keep the page compact.
    fn echo(&self) -> Value {
        let mut object = serde_json::Map::new();
        if let Some(language) = &self.language {
            let _ = object.insert("language".to_owned(), json!(language));
        }
        if let Some(bucket) = &self.bucket {
            let _ = object.insert("bucket".to_owned(), json!(bucket));
        }
        if let Some(needle) = &self.path_contains {
            let _ = object.insert("path_contains".to_owned(), json!(needle));
        }
        if let Some(score) = self.min_score {
            let _ = object.insert("min_score".to_owned(), json!(score));
        }
        if let Some(size) = self.min_size {
            let _ = object.insert("min_size".to_owned(), json!(size));
        }
        Value::Object(object)
    }
}

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

/// Builds a [`ReportPage`]-shaped JSON value over `report` with the
/// matched-and-paged cluster slice.
#[must_use]
pub fn build_page(
    report: &Report,
    generation: u64,
    pagination: Pagination,
    filters: &QueryFilters,
) -> Value {
    let matched: Vec<&ReportCluster> = report
        .clusters
        .iter()
        .filter(|cluster| filters.matches(cluster))
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
        .map(|cluster| ClusterSummary::from_report_cluster(cluster))
        .collect();
    let returned = summaries.len();

    let mut page = json!({
        "report_schema_version": report.report_schema_version,
        "tool_version": report.tool_version,
        "schema_doc": report.schema_doc,
        "generation": generation,
        "files_analysed": report.files_analysed,
        "min_nodes": report.min_nodes,
        "clusters_hidden": report.clusters_hidden,
        "embedding_provenance": report.embedding_provenance,
        "cache_stats": {
            "hits": report.cache_stats.hits,
            "misses": report.cache_stats.misses,
        },
        "metrics": report.metrics,
        "action_hints": report.action_hints,
        "total_clusters": total_clusters,
        "page": {
            "offset": pagination.offset,
            "limit": pagination.limit,
            "returned": returned,
        },
        "clusters": summaries,
    });

    let echoed = filters.echo();
    if let Value::Object(echoed_map) = &echoed {
        if !echoed_map.is_empty() {
            if let Some(page_map) = page.as_object_mut() {
                let _ = page_map.insert("filters".to_owned(), echoed);
            }
        }
    }

    page
}

impl ClusterSummary {
    /// Builds a compact page row from a full report cluster.
    fn from_report_cluster(cluster: &ReportCluster) -> Self {
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
        Self {
            id: cluster.id.clone(),
            bucket: cluster.bucket.clone(),
            score: cluster.weight,
            size_nodes: cluster.canonical_node_count,
            occurrence_count,
            language,
            first_occurrence,
        }
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
