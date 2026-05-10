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
        start_line: 1,
        end_line: 1,
        hidden: false,
    }
}

fn file_report_total_occurrences(clusters: &[ReportCluster]) -> usize {
    clusters
        .iter()
        .map(|cluster| cluster.occurrences.len())
        .sum()
}

// [LSP-SEVERITY-BUCKET] Bucket → severity mapping.
#[test]
fn severity_for_maps_bucket_to_lsp_level() {
    let identical = sample_cluster("a", 1.0, vec![occurrence("a.cs", 0, 1)], "identical");
    assert_eq!(
        severity_for(&identical),
        DiagnosticSeverity::ERROR,
        "Identical code → Error (no justification for bit-for-bit duplicates)"
    );

    let nearly = sample_cluster("b", 1.0, vec![occurrence("b.cs", 0, 1)], "nearly_identical");
    assert_eq!(
        severity_for(&nearly),
        DiagnosticSeverity::WARNING,
        "NearlyIdentical → Warning"
    );

    let loose = sample_cluster("c", 1.0, vec![occurrence("c.cs", 0, 1)], "loosely_similar");
    assert_eq!(
        severity_for(&loose),
        DiagnosticSeverity::WARNING,
        "LooselySimilar → Warning"
    );

    let behavior = sample_cluster("d", 1.0, vec![occurrence("d.cs", 0, 1)], "same_behavior");
    assert_eq!(
        severity_for(&behavior),
        DiagnosticSeverity::WARNING,
        "SameBehavior → Warning"
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

// occurrence_label removed — diagnostics now show a single "Canonical"
// link rather than an indexed list of all occurrences.

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
fn diagnostic_message_shows_category_count_and_action() {
    let cluster = sample_cluster(
        "c",
        100.0,
        vec![occurrence("a.cs", 0, 1), occurrence("b.cs", 0, 1)],
        "nearly_identical",
    );
    let message = diagnostic_message(&cluster);
    assert!(message.contains(" — "), "joined with em dash: {message}");
    assert!(
        message.contains("Nearly identical code"),
        "diagnostic message must use human label: {message}"
    );
    assert!(
        message.contains("× 2"),
        "diagnostic message must include instance count: {message}"
    );
    assert!(
        !message.contains("Type-"),
        "diagnostic message must not expose clone taxonomy labels: {message}"
    );
}

// [LSP-SEVERITY-BUCKET] Identical code → Error; canonical link present.
#[test]
fn build_for_file_emits_error_for_identical_cluster_with_canonical_link() -> Result<()> {
    let workspace = TempDir::new()?;
    let primary_source = "alpha\nbeta\ngamma\n";
    let secondary_source = "a\nbb\nccc\ndddd\n";
    let _primary = write_source(workspace.path(), "Alpha.cs", primary_source)?;
    let _secondary = write_source(workspace.path(), "Beta.cs", secondary_source)?;
    let occurrences = vec![occurrence("Alpha.cs", 0, 5), occurrence("Beta.cs", 2, 5)];
    let cluster = sample_cluster("cluster-1", 100.0, occurrences, "identical");
    let total_occurrences: usize = file_report_total_occurrences(std::slice::from_ref(&cluster));
    let file_report = FileReport {
        path: PathBuf::from("Alpha.cs"),
        clusters: vec![cluster],
        total_occurrences,
    };
    let diagnostics = build_for_file(&file_report, workspace.path());
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
        Some(DiagnosticSeverity::ERROR),
        "Identical bucket → Error per [LSP-SEVERITY-BUCKET]"
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
    assert_eq!(
        related.len(),
        1,
        "exactly one canonical link — not a full occurrence dump"
    );
    let canonical = related.first().ok_or_else(|| anyhow!("canonical entry"))?;
    assert_eq!(
        canonical.message, "Canonical",
        "related label must be 'Canonical', not an indexed occurrence string: {}",
        canonical.message
    );
    assert_eq!(
        diagnostic.range.start.line, 0,
        "start on first line of Alpha.cs"
    );
    Ok(())
}

// [LSP-SEVERITY-BUCKET] All buckets publish diagnostics — none are suppressed by default.
#[test]
fn build_for_file_publishes_all_buckets_with_correct_severity() -> Result<()> {
    let workspace = TempDir::new()?;
    let _primary = write_source(workspace.path(), "A.cs", "abc\n")?;
    let buckets = [
        ("identical", DiagnosticSeverity::ERROR),
        ("nearly_identical", DiagnosticSeverity::WARNING),
        ("loosely_similar", DiagnosticSeverity::WARNING),
        ("same_behavior", DiagnosticSeverity::WARNING),
    ];
    for (bucket, expected_severity) in buckets {
        let cluster = sample_cluster("c", 1.0, vec![occurrence("A.cs", 0, 2)], bucket);
        let total_occurrences: usize =
            file_report_total_occurrences(std::slice::from_ref(&cluster));
        let file_report = FileReport {
            path: PathBuf::from("A.cs"),
            clusters: vec![cluster],
            total_occurrences,
        };
        let diagnostics = build_for_file(&file_report, workspace.path());
        assert_eq!(
            diagnostics.len(),
            1,
            "bucket '{bucket}' must always produce a diagnostic (no weight-percentile suppression)"
        );
        let diag = diagnostics
            .first()
            .ok_or_else(|| anyhow!("no diagnostic for bucket '{bucket}'"))?;
        assert_eq!(
            diag.severity,
            Some(expected_severity),
            "bucket '{bucket}' → {expected_severity:?}"
        );
    }
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
    let total_occurrences: usize = file_report_total_occurrences(std::slice::from_ref(&cluster));
    let file_report = FileReport {
        path: PathBuf::from("Alpha.cs"),
        clusters: vec![cluster],
        total_occurrences,
    };
    let diagnostics = build_for_file(&file_report, workspace.path());
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
fn many_occurrences_produce_exactly_one_canonical_related_item() -> Result<()> {
    // The diagnostic hover must never dump a full occurrence list.
    // 38 occurrences → still exactly 1 "Canonical" related-info link.
    let workspace = TempDir::new()?;
    let primary_source = "fn a() {}\n";
    let _primary = write_source(workspace.path(), "Main.cs", primary_source)?;
    let other_source = "fn b() {}\n";
    let _other = write_source(workspace.path(), "Other.cs", other_source)?;
    let mut occs = vec![occurrence("Main.cs", 0, 5)];
    for _ in 0..37 {
        occs.push(occurrence("Other.cs", 0, 3));
    }
    let cluster = sample_cluster("big", 100.0, occs, "identical");
    let total_occurrences: usize = file_report_total_occurrences(std::slice::from_ref(&cluster));
    let file_report = FileReport {
        path: PathBuf::from("Main.cs"),
        clusters: vec![cluster],
        total_occurrences,
    };
    let diagnostics = build_for_file(&file_report, workspace.path());
    let diagnostic = diagnostics.first().ok_or_else(|| anyhow!("diagnostic"))?;
    let related = diagnostic
        .related_information
        .as_ref()
        .ok_or_else(|| anyhow!("related info must be present"))?;
    assert_eq!(
        related.len(),
        1,
        "38 occurrences must yield exactly 1 canonical link, not {}: {related:?}",
        related.len()
    );
    let canonical = related
        .first()
        .ok_or_else(|| anyhow!("related must have first entry (len asserted above)"))?;
    assert_eq!(
        canonical.message, "Canonical",
        "related label must be 'Canonical': {}",
        canonical.message
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
