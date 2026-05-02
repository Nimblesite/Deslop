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
