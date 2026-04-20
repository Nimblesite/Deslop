//! LSP diagnostic builder ([LSP-DIAGNOSTICS], [LSP-SEVERITY]).
//!
//! Translates a [`FileReport`] into the LSP `Diagnostic` shape, mapping
//! per-cluster weights onto the four severity buckets specified in
//! [LSP-SEVERITY]. Severity bucketing is computed from the percentile
//! of the cluster's weight against every other cluster in the file
//! report — clients call `textDocument/diagnostic` per file so the
//! local report is the right scope for "top-1% offender on this file".

use codededup_core::live::FileReport;
use codededup_core::report::ReportCluster;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Builds the diagnostics for one file report.
#[must_use]
pub fn build_for_file(report: &FileReport) -> Vec<Diagnostic> {
    let weights: Vec<f64> = report
        .clusters
        .iter()
        .map(|cluster| cluster.weight)
        .collect();
    report
        .clusters
        .iter()
        .flat_map(|cluster| build_for_cluster(cluster, &weights, &report.path))
        .collect()
}

/// Builds the per-cluster diagnostic — one entry per occurrence in
/// `path`.
fn build_for_cluster(
    cluster: &ReportCluster,
    weights: &[f64],
    path: &std::path::Path,
) -> Vec<Diagnostic> {
    let severity = severity_for(cluster.weight, weights);
    cluster
        .occurrences
        .iter()
        .filter(|occ| occ.path == path || occ.path.ends_with(path) || path.ends_with(&occ.path))
        .map(|occurrence| Diagnostic {
            range: byte_range_to_lsp(occurrence.start_byte, occurrence.end_byte),
            severity,
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                cluster.id.clone(),
            )),
            code_description: None,
            source: Some("codededup".to_owned()),
            message: cluster.interpretation.clone(),
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// Maps the cluster weight onto an LSP severity using the bucketing
/// in [LSP-SEVERITY]. `None` means "below the visible threshold" —
/// callers drop those.
fn severity_for(weight: f64, weights: &[f64]) -> Option<DiagnosticSeverity> {
    let percentile = percentile_for(weight, weights);
    if percentile >= 0.99 {
        Some(DiagnosticSeverity::WARNING)
    } else if percentile >= 0.90 {
        Some(DiagnosticSeverity::INFORMATION)
    } else if percentile >= 0.50 {
        Some(DiagnosticSeverity::HINT)
    } else {
        None
    }
}

/// Returns the fraction of `weights` strictly less than `weight`. A
/// cluster equal to the maximum lands at `1.0`.
fn percentile_for(weight: f64, weights: &[f64]) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }
    let lesser = weights.iter().filter(|other| **other < weight).count();
    let total = weights.len();
    let lesser_f = u32::try_from(lesser).map_or(f64::from(u32::MAX), f64::from);
    let total_f = u32::try_from(total).map_or(f64::from(u32::MAX), f64::from);
    if total_f == 0.0 {
        0.0
    } else {
        lesser_f / total_f
    }
}

/// Translates a byte range into a `Range` of zero-indexed positions.
/// We intentionally collapse byte offsets onto the same line because
/// the LSP server does not own the open buffer in this skeleton —
/// downstream clients re-derive line numbers from byte offsets via
/// the same mechanism the renderer uses.
fn byte_range_to_lsp(start_byte: usize, end_byte: usize) -> Range {
    Range {
        start: Position {
            line: 0,
            character: u32::try_from(start_byte).unwrap_or(u32::MAX),
        },
        end: Position {
            line: 0,
            character: u32::try_from(end_byte).unwrap_or(u32::MAX),
        },
    }
}
