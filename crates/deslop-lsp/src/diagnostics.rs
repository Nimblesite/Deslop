//! LSP diagnostic builder ([LSP-DIAGNOSTICS], [LSP-SEVERITY]).
//!
//! Translates a [`FileReport`] into the LSP `Diagnostic` shape.
//! Severity is determined by clone bucket per [LSP-SEVERITY-BUCKET]:
//!   `Identical` → `Error`, all others → `Warning` by default.
//! Configurable per-bucket via `deslop.severity.*` settings; future
//! percentile-floor support is specced in [LSP-SEVERITY-PERCENTILE].
//!
//! Occurrence paths in the report are workspace-relative; this module
//! resolves them against the session's workspace root before
//! constructing `file://` URLs so `relatedInformation` jumps land on
//! real files. Byte offsets are translated to `(line, character)` LSP
//! positions by reading the source text and counting UTF-16 code units.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use deslop_core::buckets::{classify, ClusterKind};
use deslop_core::live::FileReport;
use deslop_core::report::{ReportCluster, ReportOccurrence};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range, Url,
};

use crate::position::position_for_byte;
use crate::presentation::{diagnostic_data, diagnostic_message};

/// Builds the diagnostics for one file report ([LSP-DIAGNOSTICS]).
///
/// `workspace_root` is the absolute path the session was rooted at;
/// relative occurrence paths are resolved against it so
/// `relatedInformation` URLs are valid.
#[must_use]
pub fn build_for_file(report: &FileReport, workspace_root: &Path) -> Vec<Diagnostic> {
    let primary_path = absolute_path(&report.path, workspace_root);
    let primary_source = std::fs::read_to_string(&primary_path).unwrap_or_default();
    let mut source_cache: HashMap<PathBuf, String> = HashMap::new();
    report
        .clusters
        .iter()
        .flat_map(|cluster| {
            build_for_cluster(
                cluster,
                &report.path,
                workspace_root,
                &primary_source,
                &mut source_cache,
            )
        })
        .collect()
}

/// Resolves `path` against `workspace_root` when it is relative.
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

/// Builds the per-cluster diagnostic — one entry per occurrence in `path`.
fn build_for_cluster(
    cluster: &ReportCluster,
    path: &Path,
    workspace_root: &Path,
    source_bytes: &str,
    cache: &mut HashMap<PathBuf, String>,
) -> Vec<Diagnostic> {
    let severity = severity_for(cluster);
    cluster
        .occurrences
        .iter()
        .filter(|occ| occurrence_matches_path(occ, path))
        .map(|occurrence| Diagnostic {
            range: byte_range_to_lsp(occurrence.start_byte, occurrence.end_byte, source_bytes),
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("deslop".to_owned()),
            message: diagnostic_message(cluster),
            related_information: related_info_for(cluster, path, workspace_root, cache),
            tags: None,
            data: Some(diagnostic_data(cluster)),
        })
        .collect()
}

/// Returns `true` when the occurrence matches the file the report applies to.
pub(crate) fn occurrence_matches_path(occurrence: &ReportOccurrence, path: &Path) -> bool {
    occurrence.path == path || occurrence.path.ends_with(path) || path.ends_with(&occurrence.path)
}

/// Maps cluster bucket → LSP severity per [LSP-SEVERITY-BUCKET].
/// Defaults: `Identical` → `Error`, `StructuralOnly` → `Hint`
/// (shape-only evidence, demoted in ranking per
/// [RANK-STRUCTURAL-ONLY]), all others → `Warning`.
/// Future: configurable per bucket via `deslop.severity.*` settings.
fn severity_for(cluster: &ReportCluster) -> DiagnosticSeverity {
    match classify(cluster) {
        ClusterKind::Identical => DiagnosticSeverity::ERROR,
        ClusterKind::StructuralOnly => DiagnosticSeverity::HINT,
        ClusterKind::NearlyIdentical | ClusterKind::LooselySimilar | ClusterKind::SameBehavior => {
            DiagnosticSeverity::WARNING
        }
    }
}

/// Translates a byte range into a zero-indexed LSP `Range`.
fn byte_range_to_lsp(start_byte: usize, end_byte: usize, source_bytes: &str) -> Range {
    Range {
        start: position_for_byte(source_bytes, start_byte),
        end: position_for_byte(source_bytes, end_byte),
    }
}

/// Returns a single `relatedInformation` entry pointing to the canonical
/// occurrence — the first occurrence of the cluster not in `path`.
/// Showing only the canonical keeps the diagnostic hover minimal.
fn related_info_for(
    cluster: &ReportCluster,
    path: &Path,
    workspace_root: &Path,
    cache: &mut HashMap<PathBuf, String>,
) -> Option<Vec<DiagnosticRelatedInformation>> {
    let canonical = cluster
        .occurrences
        .iter()
        .find(|occ| !occurrence_matches_path(occ, path))?;
    canonical_item(canonical, workspace_root, cache).map(|item| vec![item])
}

/// Constructs the canonical `DiagnosticRelatedInformation` entry.
fn canonical_item(
    occurrence: &ReportOccurrence,
    workspace_root: &Path,
    cache: &mut HashMap<PathBuf, String>,
) -> Option<DiagnosticRelatedInformation> {
    let absolute = absolute_path(&occurrence.path, workspace_root);
    let uri = Url::from_file_path(&absolute).ok()?;
    let source = load_cached_source(&absolute, cache);
    let range = byte_range_to_lsp(occurrence.start_byte, occurrence.end_byte, &source);
    Some(DiagnosticRelatedInformation {
        location: Location { uri, range },
        message: "Canonical".to_owned(),
    })
}

/// Public helper for the code-lens layer.
#[must_use]
pub fn byte_range(start_byte: usize, end_byte: usize, source: &str) -> Range {
    byte_range_to_lsp(start_byte, end_byte, source)
}

/// Exposed helper for tests.
#[must_use]
pub fn position_at(source: &str, byte_offset: usize) -> Position {
    position_for_byte(source, byte_offset)
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
#[path = "diagnostics_tests.rs"]
mod tests;
