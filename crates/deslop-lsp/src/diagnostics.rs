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
//! [`deslop_core::live::LiveApi::all_cluster_weights`].
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

use deslop_core::live::FileReport;
use deslop_core::report::{ReportCluster, ReportOccurrence};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range, Url,
};

use crate::position::position_for_byte;
use crate::presentation::{diagnostic_data, diagnostic_message};

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

/// Returns `true` when the occurrence matches the file the report
/// applies to. Handles the common relative/absolute skew.
fn occurrence_matches_path(occurrence: &ReportOccurrence, path: &Path) -> bool {
    occurrence.path == path || occurrence.path.ends_with(path) || path.ends_with(&occurrence.path)
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
    lesser_f / total_f
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

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use deslop_core::report::ReportSignals;
    use tempfile::TempDir;

    fn write_source(dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
        let path = dir.join(name);
        std::fs::write(&path, body)?;
        Ok(path)
    }

    fn sample_cluster(
        id: &str,
        weight: f64,
        occurrences: Vec<ReportOccurrence>,
        bucket: &str,
    ) -> ReportCluster {
        ReportCluster {
            id: id.to_owned(),
            weight,
            size: occurrences.len(),
            canonical_node_count: 25,
            signals: ReportSignals {
                structural: 1.0,
                token_jaccard: 0.9,
                embedding_cos: 0.4,
                fused: 2.2,
            },
            bucket: bucket.into(),
            occurrences_total: occurrences.len(),
            occurrences_truncated: false,
            occurrences,
            summary: "summary".into(),
            interpretation: "interp".into(),
        }
    }

    fn occurrence(path: &str, start: usize, end: usize) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: start,
            end_byte: end,
            hidden: false,
        }
    }

    #[test]
    fn percentile_for_orders_below_equal_and_above_correctly() {
        let weights = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(
            (percentile_for(5.0, &weights) - 0.8).abs() < f64::EPSILON,
            "max lands at (n-1)/n"
        );
        assert!(
            (percentile_for(1.0, &weights)).abs() < f64::EPSILON,
            "min lands at 0"
        );
        assert!(
            (percentile_for(3.0, &weights) - 0.4).abs() < f64::EPSILON,
            "median-ish lands at 0.4"
        );
        assert!(
            (percentile_for(10.0, &weights) - 1.0).abs() < f64::EPSILON,
            "above max lands at 1.0"
        );
        assert!(
            (percentile_for(7.0, &[])).abs() < f64::EPSILON,
            "empty weights clamps to 0"
        );
    }

    #[test]
    fn severity_for_maps_percentiles_to_lsp_buckets() {
        let weights: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(
            severity_for(100.0, &weights),
            Some(DiagnosticSeverity::WARNING),
            "top 1% → WARNING"
        );
        assert_eq!(
            severity_for(95.0, &weights),
            Some(DiagnosticSeverity::INFORMATION),
            "top 10% → INFORMATION"
        );
        assert_eq!(
            severity_for(60.0, &weights),
            Some(DiagnosticSeverity::HINT),
            "top 50% → HINT"
        );
        assert_eq!(
            severity_for(10.0, &weights),
            None,
            "bottom half → suppressed"
        );
    }

    #[test]
    fn absolute_path_leaves_absolute_untouched_and_joins_relative() {
        let workspace = Path::new("/ws");
        let absolute = PathBuf::from("/other/root/Alpha.cs");
        assert_eq!(
            absolute_path(&absolute, workspace),
            absolute,
            "absolute paths pass through unchanged"
        );
        let relative = PathBuf::from("src/Beta.cs");
        assert_eq!(
            absolute_path(&relative, workspace),
            PathBuf::from("/ws/src/Beta.cs"),
            "relative paths are joined against workspace root"
        );
    }

    #[test]
    fn occurrence_matches_path_handles_relative_absolute_skew() {
        let absolute = Path::new("/ws/src/Alpha.cs");
        let relative_occ = occurrence("src/Alpha.cs", 0, 1);
        assert!(occurrence_matches_path(&relative_occ, absolute));
        let absolute_occ = occurrence("/ws/src/Alpha.cs", 0, 1);
        assert!(occurrence_matches_path(
            &absolute_occ,
            Path::new("src/Alpha.cs")
        ));
        let unrelated = occurrence("Gamma.cs", 0, 1);
        assert!(!occurrence_matches_path(&unrelated, Path::new("Delta.cs")));
    }

    #[test]
    fn byte_range_to_lsp_spans_newlines_and_utf16() {
        let source = "abc\ndef\nghij";
        let range = byte_range_to_lsp(1, 9, source);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 1);
        assert_eq!(range.end.line, 2);
        assert_eq!(range.end.character, 1);
        let also = byte_range(1, 9, source);
        assert_eq!(
            also, range,
            "public byte_range helper delegates to the same implementation"
        );
        let at = position_at(source, 4);
        assert_eq!(at.line, 1);
        assert_eq!(at.character, 0);
    }

    #[test]
    fn occurrence_label_uses_one_based_indexing() {
        assert_eq!(occurrence_label(0, 3), "occurrence 1 of 3");
        assert_eq!(occurrence_label(1, 3), "occurrence 2 of 3");
        assert_eq!(occurrence_label(4, 5), "occurrence 5 of 5");
        // saturating_add on usize::MAX clamps rather than panicking.
        let saturated = occurrence_label(usize::MAX, 1);
        assert!(
            saturated.ends_with(" of 1"),
            "total still trails after saturation: {saturated}"
        );
        assert!(
            saturated.starts_with("occurrence "),
            "label prefix preserved even under saturation: {saturated}"
        );
    }

    #[test]
    fn diagnostic_data_stores_cluster_id_for_machine_readers() -> Result<()> {
        let cluster = sample_cluster(
            "abc123",
            10.0,
            vec![occurrence("Alpha.cs", 0, 5)],
            "identical",
        );
        let data = diagnostic_data(&cluster);
        let id = data
            .get("cluster_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("cluster_id in diagnostic data"))?;
        assert_eq!(id, "abc123");
        Ok(())
    }

    #[test]
    fn diagnostic_message_uses_plain_title_and_action_sentence() {
        let cluster = sample_cluster(
            "c",
            100.0,
            vec![occurrence("a.cs", 0, 1)],
            "nearly_identical",
        );
        let message = diagnostic_message(&cluster);
        assert!(message.contains(" — "), "joined with em dash: {message}");
        assert!(
            message.contains("Nearly identical code"),
            "diagnostic message must use human label: {message}"
        );
        assert!(
            !message.contains("Type-"),
            "diagnostic message must not expose clone taxonomy labels: {message}"
        );
        assert!(
            !message.is_empty(),
            "non-empty diagnostic message: {message}"
        );
    }

    #[test]
    fn build_for_file_emits_diagnostic_with_relatedinfo_and_severity() -> Result<()> {
        let workspace = TempDir::new()?;
        let primary_source = "alpha\nbeta\ngamma\n";
        let secondary_source = "a\nbb\nccc\ndddd\n";
        let _primary = write_source(workspace.path(), "Alpha.cs", primary_source)?;
        let _secondary = write_source(workspace.path(), "Beta.cs", secondary_source)?;
        let occurrences = vec![occurrence("Alpha.cs", 0, 5), occurrence("Beta.cs", 2, 5)];
        let cluster = sample_cluster("cluster-1", 100.0, occurrences, "identical");
        let file_report = FileReport {
            path: PathBuf::from("Alpha.cs"),
            clusters: vec![cluster],
        };
        // 99 weights all strictly below 100.0, plus the cluster weight
        // itself: lesser = 99, total = 100, percentile = 0.99 → WARNING.
        let mut weights_with_primary: Vec<f64> = (1..=99).map(f64::from).collect();
        weights_with_primary.push(100.0);
        let diagnostics = build_for_file(&file_report, &weights_with_primary, workspace.path());
        assert_eq!(
            diagnostics.len(),
            1,
            "one diagnostic for the Alpha.cs occurrence"
        );
        let diagnostic = diagnostics
            .first()
            .ok_or_else(|| anyhow!("diagnostic present"))?;
        assert_eq!(diagnostic.source.as_deref(), Some("deslop"));
        assert_eq!(
            diagnostic.severity,
            Some(DiagnosticSeverity::WARNING),
            "top percentile → WARNING"
        );
        assert!(
            diagnostic.code.is_none(),
            "cluster hash must not be visible as deslop(<id>) in editor hovers"
        );
        let cluster_id = diagnostic
            .data
            .as_ref()
            .and_then(|data| data.get("cluster_id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("cluster id stored in diagnostic data"))?;
        assert_eq!(cluster_id, "cluster-1");
        let related = diagnostic
            .related_information
            .as_ref()
            .ok_or_else(|| anyhow!("related info for Beta.cs"))?;
        assert_eq!(related.len(), 1, "only Beta.cs surfaces as related");
        let related_first = related.first().ok_or_else(|| anyhow!("first related"))?;
        assert!(
            related_first.message.contains("occurrence 2 of 2"),
            "label uses 1-based index: {}",
            related_first.message
        );
        assert_eq!(
            diagnostic.range.start.line, 0,
            "start on first line of Alpha.cs"
        );
        Ok(())
    }

    #[test]
    fn build_for_file_drops_clusters_with_suppressed_severity() -> Result<()> {
        let workspace = TempDir::new()?;
        let _primary = write_source(workspace.path(), "Alpha.cs", "abc\n")?;
        let cluster = sample_cluster(
            "cluster-low",
            1.0,
            vec![occurrence("Alpha.cs", 0, 2)],
            "loosely_similar",
        );
        let file_report = FileReport {
            path: PathBuf::from("Alpha.cs"),
            clusters: vec![cluster],
        };
        let weights: Vec<f64> = (50..=100).map(f64::from).collect();
        let diagnostics = build_for_file(&file_report, &weights, workspace.path());
        assert!(
            diagnostics.is_empty(),
            "weight below 50th percentile → dropped: {diagnostics:?}"
        );
        Ok(())
    }

    #[test]
    fn build_for_file_empty_related_info_becomes_none() -> Result<()> {
        let workspace = TempDir::new()?;
        let _primary = write_source(workspace.path(), "Alpha.cs", "abcdef\n")?;
        let cluster = sample_cluster(
            "solo",
            100.0,
            vec![occurrence("Alpha.cs", 0, 3)],
            "identical",
        );
        let file_report = FileReport {
            path: PathBuf::from("Alpha.cs"),
            clusters: vec![cluster],
        };
        let weights = vec![1.0_f64, 2.0, 100.0];
        let diagnostics = build_for_file(&file_report, &weights, workspace.path());
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = diagnostics
            .first()
            .ok_or_else(|| anyhow!("diagnostic present"))?;
        assert!(
            diagnostic.related_information.is_none(),
            "no other occurrences → related_information is None"
        );
        Ok(())
    }

    #[test]
    fn load_cached_source_reuses_cache_and_survives_missing_files() -> Result<()> {
        let workspace = TempDir::new()?;
        let real = write_source(workspace.path(), "Real.cs", "hello\n")?;
        let mut cache: HashMap<PathBuf, String> = HashMap::new();
        let first = load_cached_source(&real, &mut cache);
        assert_eq!(first, "hello\n");
        assert!(cache.contains_key(&real), "entry cached after first read");
        let missing = workspace.path().join("missing.cs");
        let body = load_cached_source(&missing, &mut cache);
        assert!(
            body.is_empty(),
            "missing files fall back to empty string, not panic"
        );
        let second = load_cached_source(&real, &mut cache);
        assert_eq!(second, "hello\n", "cached read returns same content");
        Ok(())
    }
}
