//! Path and cluster filtering helpers for the MCP backend.

use std::path::Path;

use deslop_core::{
    report::{ReportCluster, ReportOccurrence},
    Report,
};

/// Filters the report down to clusters whose occurrences touch
/// `absolute_candidate`.
pub(super) fn filter_clusters_by_path(
    report: &Report,
    absolute_candidate: &Path,
    root: &Path,
) -> Vec<ReportCluster> {
    report
        .clusters
        .iter()
        .filter(|cluster| {
            cluster
                .occurrences
                .iter()
                .any(|occ| paths_equal(&occ.path, absolute_candidate, root))
        })
        .cloned()
        .collect()
}

/// Filters the report to clusters overlapping
/// `[start_byte, end_byte)` on `absolute_candidate`.
pub(super) fn filter_clusters_by_range(
    report: &Report,
    absolute_candidate: &Path,
    start_byte: usize,
    end_byte: usize,
    root: &Path,
) -> Vec<ReportCluster> {
    report
        .clusters
        .iter()
        .filter(|cluster| {
            cluster.occurrences.iter().any(|occ| {
                paths_equal(&occ.path, absolute_candidate, root)
                    && occurrence_overlaps(occ, start_byte, end_byte)
            })
        })
        .cloned()
        .collect()
}

/// Returns whether `occ` overlaps `[start_byte, end_byte)`.
pub(super) const fn occurrence_overlaps(
    occ: &ReportOccurrence,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    occ.start_byte < end_byte && occ.end_byte > start_byte
}

/// Compares an occurrence path (stored relative to the scan root by
/// the renderer) against an absolute path. The renderer stores
/// scan-root-relative paths, so we reconstruct the absolute form by
/// canonicalising `root.join(occ)` and match against the canonical
/// candidate.
pub(super) fn paths_equal(occurrence_path: &Path, absolute_candidate: &Path, root: &Path) -> bool {
    let joined = root.join(occurrence_path);
    std::fs::canonicalize(&joined).is_ok_and(|canonical| canonical == absolute_candidate)
}
