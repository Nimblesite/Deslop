//! [MCP-TOOL-FILTERS-KINDS] One effective selection for every cluster-returning tool.

use deslop_core::buckets::TotalInclude;

use super::{cluster_language, DuplicatesFilters, Report, ReportCluster};

/// Resolves omitted kinds from the engine's configuration and normalises overrides.
pub(super) fn effective_filters(
    report: &Report,
    requested: &DuplicatesFilters,
) -> DuplicatesFilters {
    let mut filters = requested.clone();
    let kinds = requested
        .kinds
        .clone()
        .unwrap_or_else(|| report.metrics.total_includes.clone());
    filters.kinds = Some(TotalInclude::from_kinds(kinds).kinds().to_vec());
    filters
}

/// Applies every cluster-owned filter before pagination and occurrence budgeting.
pub(super) fn matches_filters(filters: &DuplicatesFilters, cluster: &ReportCluster) -> bool {
    filters
        .kinds
        .as_ref()
        .map_or(true, |kinds| kinds.contains(&cluster.kind))
        && filters
            .min_size
            .map_or(true, |minimum| cluster.canonical_node_count >= minimum)
        && matches_values(filters.severities.as_deref(), |value| {
            value == &cluster.severity
        })
        && matches_values(filters.languages.as_deref(), |value| {
            value == cluster_language(cluster)
        })
        && filters.path_contains.as_ref().map_or(true, |needle| {
            cluster
                .occurrences
                .iter()
                .any(|item| item.path.to_string_lossy().contains(needle))
        })
}

/// Empty language and severity arrays impose no restriction ([MCP-TOOL-FILTERS]).
fn matches_values(values: Option<&[String]>, matches: impl Fn(&String) -> bool) -> bool {
    values.map_or(true, |values| {
        values.is_empty() || values.iter().any(matches)
    })
}
