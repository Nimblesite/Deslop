//! Paginated mass-only duplicate-cluster pages.

use deslop_core::{
    pipeline::language_for_path,
    report::{occurrence_count, Report, ReportCluster},
    wire_generated::{
        ClusterSummary, DuplicateCluster, DuplicatesFilters, DuplicatesPage, OccurrenceSummary,
        RepoMetrics, ReportPageInfo,
    },
};

/// Page cursor and optional metrics detail.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Zero-based matched-cluster offset.
    pub offset: usize,
    /// Maximum returned clusters.
    pub limit: usize,
    /// Whether per-file metrics are included.
    pub include_per_file: bool,
}

/// Wire detail selected by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Full cluster records with an occurrence budget.
    Full,
    /// Compact cluster summaries without occurrences.
    Summary,
}

/// Inputs that control a duplicates page.
#[derive(Debug, Clone, Copy)]
pub struct PageShape {
    /// Pagination cursor.
    pub pagination: Pagination,
    /// Summary or full rows.
    pub detail: Detail,
    /// Total occurrence budget for full rows.
    pub max_occurrences: usize,
}

/// Builds a mass-ranked duplicate page from already-scoped candidates.
#[must_use]
pub fn build_page(
    report: &Report,
    generation: u64,
    candidates: &[ReportCluster],
    shape: PageShape,
    filters: &DuplicatesFilters,
) -> DuplicatesPage {
    let matched: Vec<&ReportCluster> = candidates
        .iter()
        .filter(|cluster| matches_filters(filters, cluster))
        .collect();
    let total_clusters = matched.len();
    let total_occurrences = matched
        .iter()
        .map(|cluster| occurrence_count(cluster))
        .sum();
    let clusters = selected_page(&matched, shape);
    DuplicatesPage {
        tool_version: report.tool_version.clone(),
        generation,
        files_analysed: report.files_analysed,
        min_nodes: report.min_nodes,
        clusters_hidden: report.clusters_hidden,
        embedding_provenance: report.embedding_provenance.clone(),
        cache_stats: report.cache_stats,
        metrics: page_metrics(report, shape.pagination.include_per_file),
        total_clusters,
        total_occurrences,
        page: ReportPageInfo {
            offset: shape.pagination.offset,
            limit: shape.pagination.limit,
            returned: clusters.len(),
        },
        clusters,
        filters: filters.clone(),
    }
}

/// Selects, projects, and budgets the requested page.
fn selected_page(matched: &[&ReportCluster], shape: PageShape) -> Vec<DuplicateCluster> {
    let start = shape.pagination.offset.min(matched.len());
    let end = start
        .saturating_add(shape.pagination.limit)
        .min(matched.len());
    let selected = matched.get(start..end).unwrap_or_default();
    match shape.detail {
        Detail::Summary => selected
            .iter()
            .map(|cluster| DuplicateCluster::Summary(cluster_summary(cluster)))
            .collect(),
        Detail::Full => budgeted_full_rows(selected, shape.max_occurrences),
    }
}

/// Applies the total occurrence budget to full rows.
fn budgeted_full_rows(clusters: &[&ReportCluster], budget: usize) -> Vec<DuplicateCluster> {
    let mut used = 0_usize;
    clusters
        .iter()
        .map_while(|cluster| budget_cluster(cluster, budget, &mut used))
        .map(DuplicateCluster::Full)
        .collect()
}

/// Copies one cluster with at most the remaining occurrence budget.
fn budget_cluster(
    cluster: &ReportCluster,
    budget: usize,
    used: &mut usize,
) -> Option<ReportCluster> {
    if *used >= budget {
        return None;
    }
    let mut copy = cluster.clone();
    copy.occurrences_total = occurrence_count(&copy);
    let remaining = budget.saturating_sub(*used);
    if copy.occurrences.len() > remaining {
        copy.occurrences.truncate(remaining);
        copy.occurrences_truncated = true;
    }
    *used = used.saturating_add(copy.occurrences.len());
    Some(copy)
}

/// Removes large per-file metrics unless explicitly requested.
fn page_metrics(report: &Report, include_per_file: bool) -> RepoMetrics {
    let mut metrics = report.metrics.clone();
    if !include_per_file {
        metrics.per_file.clear();
        metrics.folders.clear();
    }
    metrics
}

/// Applies only cluster-owned filters.
fn matches_filters(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    filter_min_size(filters, cluster)
        && filter_severity(filters, cluster)
        && filter_language(filters, cluster)
        && filter_path(filters, cluster)
}

/// Applies the canonical extent floor.
fn filter_min_size(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    match filters.min_size {
        Some(minimum) => cluster.canonical_node_count >= minimum,
        None => true,
    }
}

/// Applies engine-stamped mass severity bands.
fn filter_severity(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    match filters.severities.as_ref() {
        Some(values) => values.iter().any(|value| value == &cluster.rank_band),
        None => true,
    }
}

/// Applies language filters derived from the stable first occurrence.
fn filter_language(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    match filters.languages.as_ref() {
        Some(values) => values
            .iter()
            .any(|value| value == cluster_language(cluster)),
        None => true,
    }
}

/// Applies occurrence-path substring filtering.
fn filter_path(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    match filters.path_contains.as_ref() {
        Some(needle) => cluster
            .occurrences
            .iter()
            .any(|occurrence| occurrence.path.to_string_lossy().contains(needle)),
        None => true,
    }
}

/// Returns the stable first occurrence's language id.
fn cluster_language(cluster: &ReportCluster) -> &'static str {
    cluster
        .occurrences
        .first()
        .map_or("unknown", |occurrence| language_for_path(&occurrence.path))
}

/// Builds a compact mass-only cluster row.
fn cluster_summary(cluster: &ReportCluster) -> ClusterSummary {
    ClusterSummary {
        id: cluster.id.clone(),
        rank: cluster.rank,
        rank_band: cluster.rank_band.clone(),
        mass: cluster.mass,
        size_nodes: cluster.canonical_node_count,
        occurrence_count: cluster.occurrence_count,
        language: cluster_language(cluster).to_owned(),
        first_occurrence: cluster.occurrences.first().map(occurrence_summary),
    }
}

/// Builds one navigation occurrence.
fn occurrence_summary(occurrence: &deslop_core::report::ReportOccurrence) -> OccurrenceSummary {
    OccurrenceSummary {
        path: occurrence.path.to_string_lossy().into_owned(),
        start_byte: occurrence.start_byte,
        end_byte: occurrence.end_byte,
        start_line: occurrence.start_line,
        end_line: occurrence.end_line,
    }
}
