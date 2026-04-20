//! LSP diagnostic builder ([LSP-DIAGNOSTICS], [LSP-SEVERITY]).
//!
//! Translates a [`FileReport`] into the LSP `Diagnostic` shape, mapping
//! per-cluster weights onto the four severity buckets specified in
//! [LSP-SEVERITY]. Severity bucketing uses the percentile of the
//! cluster's weight against the **whole live report**, not just the
//! current file: a cluster that is the worst in a sleepy file but
//! mid-tier overall must rank mid-tier in the Problems panel —
//! agreeing with the top-offenders tree, the CLI text report, and the
//! HTML report. Callers obtain the global distribution through
//! [`codededup_core::live::LiveApi::all_cluster_weights`].
//!
//! Occurrence paths in the report are workspace-relative; this module
//! resolves them against the session's workspace root before
//! constructing `file://` URLs so `relatedInformation` jumps land on
//! real files.
//!
//! Byte offsets are translated to `(line, character)` LSP positions by
//! reading the source text and counting UTF-16 code units per LSP spec.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use codededup_core::buckets::{bucket_labels, classify};
use codededup_core::live::FileReport;
use codededup_core::report::{ReportCluster, ReportOccurrence};
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    Position, Range, Url,
};

use crate::position::position_for_byte;

/// Builds the diagnostics for one file report ([LSP-DIAGNOSTICS]).
///
/// `global_weights` carries the weight of every cluster in the live
/// report and drives the per-report percentile bucketing in
/// [LSP-SEVERITY]. `workspace_root` is the absolute path the session
/// was rooted at; relative occurrence paths are resolved against it so
/// `relatedInformation` URLs are valid.
#[must_use]
pub fn build_for_file(
    report: &FileReport,
    global_weights: &[f64],
    workspace_root: &Path,
) -> Vec<Diagnostic> {
    let primary_path = absolute_path(&report.path, workspace_root);
    let primary_source = std::fs::read_to_string(&primary_path).unwrap_or_default();
    let mut source_cache: HashMap<PathBuf, String> = HashMap::new();
    report
        .clusters
        .iter()
        .flat_map(|cluster| {
            build_for_cluster(
                cluster,
                global_weights,
                &report.path,
                workspace_root,
                &primary_source,
                &mut source_cache,
            )
        })
        .collect()
}

/// Resolves `path` against `workspace_root` when it is relative,
/// returning the path unchanged when it is already absolute.
fn absolute_path(path: &Path, workspace_root: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

/// Loads a file's text, caching per [`build_for_file`] invocation.
fn load_cached_source(path: &Path, cache: &mut HashMap<PathBuf, String>) -> String {
    if let Some(existing) = cache.get(path) {
        return existing.clone();
    }
    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        tracing::warn!(path = %path.display(), %error, "failed to read occurrence source");
        String::new()
    });
    let _previous = cache.insert(path.to_path_buf(), content.clone());
    content
}

/// Builds the per-cluster diagnostic — one entry per occurrence in
/// `path`.
fn build_for_cluster(
    cluster: &ReportCluster,
    global_weights: &[f64],
    path: &Path,
    workspace_root: &Path,
    source_bytes: &str,
    cache: &mut HashMap<PathBuf, String>,
) -> Vec<Diagnostic> {
    let severity = severity_for(cluster.weight, global_weights);
    if severity.is_none() {
        return Vec::new();
    }
    cluster
        .occurrences
        .iter()
        .filter(|occ| occurrence_matches_path(occ, path))
        .map(|occurrence| Diagnostic {
            range: byte_range_to_lsp(occurrence.start_byte, occurrence.end_byte, source_bytes),
            severity,
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                cluster.id.clone(),
            )),
            code_description: code_description_for(&cluster.id),
            source: Some("codededup".to_owned()),
            message: diagnostic_message(cluster),
            related_information: related_info_for(cluster, path, workspace_root, cache),
            tags: None,
            data: None,
        })
        .collect()
}

/// Returns `true` when the occurrence matches the file the report
/// applies to. Handles the common relative/absolute skew.
fn occurrence_matches_path(occurrence: &ReportOccurrence, path: &Path) -> bool {
    occurrence.path == path || occurrence.path.ends_with(path) || path.ends_with(&occurrence.path)
}

/// Builds the diagnostic message shown in the Problems panel / hover /
/// on-hover balloons. These are **shared-text** surfaces per
/// [CLONE-BUCKETS-DUAL-LABEL]: humans read them, AI agents scrape them.
/// So the message uses the hybrid form — plain title with a bracketed
/// taxonomy suffix — plus the canonical action sentence so the reader
/// always sees what to do next.
fn diagnostic_message(cluster: &ReportCluster) -> String {
    let labels = bucket_labels(classify(cluster));
    format!("{} — {}", labels.hybrid_title, labels.action_sentence)
}

/// Builds the `codededup://cluster/<id>` href.
fn code_description_for(cluster_id: &str) -> Option<CodeDescription> {
    let href = format!("codededup://cluster/{cluster_id}");
    match Url::parse(&href) {
        Ok(url) => Some(CodeDescription { href: url }),
        Err(error) => {
            tracing::warn!(%error, cluster_id, "invalid cluster href");
            None
        }
    }
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

/// Translates a byte range in `source_bytes` into a zero-indexed
/// `Range` per the LSP spec.
fn byte_range_to_lsp(start_byte: usize, end_byte: usize, source_bytes: &str) -> Range {
    Range {
        start: position_for_byte(source_bytes, start_byte),
        end: position_for_byte(source_bytes, end_byte),
    }
}

/// Builds `relatedInformation` links for the cluster's other occurrences.
fn related_info_for(
    cluster: &ReportCluster,
    path: &Path,
    workspace_root: &Path,
    cache: &mut HashMap<PathBuf, String>,
) -> Option<Vec<DiagnosticRelatedInformation>> {
    let total = cluster.occurrences.len();
    let mut items: Vec<DiagnosticRelatedInformation> = Vec::new();
    for (index, occurrence) in cluster.occurrences.iter().enumerate() {
        if occurrence_matches_path(occurrence, path) {
            continue;
        }
        if let Some(info) = related_item(index, total, occurrence, workspace_root, cache) {
            items.push(info);
        }
    }
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Constructs one `DiagnosticRelatedInformation` entry.
fn related_item(
    index: usize,
    total: usize,
    occurrence: &ReportOccurrence,
    workspace_root: &Path,
    cache: &mut HashMap<PathBuf, String>,
) -> Option<DiagnosticRelatedInformation> {
    let absolute = absolute_path(&occurrence.path, workspace_root);
    let uri = Url::from_file_path(&absolute).ok()?;
    let source = load_cached_source(&absolute, cache);
    let range = byte_range_to_lsp(occurrence.start_byte, occurrence.end_byte, &source);
    let label = occurrence_label(index, total);
    Some(DiagnosticRelatedInformation {
        location: Location { uri, range },
        message: label,
    })
}

/// Formats the "occurrence N of M" label. Uses 1-based indexing for
/// user-facing strings.
fn occurrence_label(index: usize, total: usize) -> String {
    let one_based = index.saturating_add(1);
    format!("occurrence {one_based} of {total}")
}

/// Public helper for callers that need direct access to the range
/// converter (e.g. code-lens layer).
#[must_use]
pub fn byte_range(start_byte: usize, end_byte: usize, source: &str) -> Range {
    byte_range_to_lsp(start_byte, end_byte, source)
}

/// Exposed helper for tests.
#[must_use]
pub fn position_at(source: &str, byte_offset: usize) -> Position {
    position_for_byte(source, byte_offset)
}
