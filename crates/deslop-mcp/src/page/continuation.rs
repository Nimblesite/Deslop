//! [MCP-PAGE-CONTINUATION] Cursors follow the rows actually returned.

use deslop_core::wire_generated::{DuplicateCluster, ReportPageInfo};

/// Computes one cursor after either occurrence budgeting or byte-cap trimming.
pub(crate) fn page_info(
    offset: usize,
    limit: usize,
    returned: usize,
    total: usize,
) -> ReportPageInfo {
    let next = offset.saturating_add(returned).min(total);
    let has_more = next < total;
    ReportPageInfo {
        offset,
        limit,
        returned,
        next_offset: has_more.then_some(next),
        has_more,
    }
}

/// A normal paginated subset is complete for its limit; a budget-limited one is not.
pub(super) fn is_truncated(rows: &[DuplicateCluster], page: &ReportPageInfo, total: usize) -> bool {
    let requested = total.saturating_sub(page.offset).min(page.limit);
    rows.len() < requested
        || rows.iter().any(
            |row| matches!(row, DuplicateCluster::Full(cluster) if cluster.occurrences_truncated),
        )
}
