use super::*;
use anyhow::{anyhow, Result};
use deslop_core::report::ReportSignals;
use tempfile::TempDir;

// [LSP-SEVERITY-BUCKET] Every bucket, the severity it must publish, and the
// rationale that mapping pins.
const BUCKET_SEVERITIES: [(&str, DiagnosticSeverity, &str); 4] = [
    (
        "identical",
        DiagnosticSeverity::ERROR,
        "Identical code → Error (no justification for bit-for-bit duplicates)",
    ),
    (
        "nearly_identical",
        DiagnosticSeverity::WARNING,
        "NearlyIdentical → Warning",
    ),
    (
        "loosely_similar",
        DiagnosticSeverity::WARNING,
        "LooselySimilar → Warning",
    ),
    (
        "same_behavior",
        DiagnosticSeverity::WARNING,
        "SameBehavior → Warning",
    ),
];

// [FUSION-CONTENT-GATE] #344: each measured axis a diagnostic built from the
// `sample_cluster` signals must state, with the evidence that axis carries.
const SAMPLE_EVIDENCE_AXES: [(&str, &str); 7] = [
    ("structural 1.00", "structural axis"),
    ("jaccard 0.90", "token axis"),
    ("embedding 0.40", "embedding axis"),
    ("fused 0.91", "fused confidence"),
    ("agreement 0.58", "pooled byte agreement"),
    ("rename 0.72", "Baker rename corroboration"),
    ("literal 0.24", "literal share of the match"),
];

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
            fused: 0.91,
            agreement: 0.58,
            rename_consistency: 0.72,
            literal_fraction: 0.24,
        },
        bucket: bucket.into(),
        category: "logic".into(),
        occurrences_total: occurrences.len(),
        occurrences_truncated: false,
        occurrences,
        summary: "summary".into(),
        interpretation: "interp".into(),
        intersects_diff: None,
        is_newly_introduced: None,
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
        in_diff: None,
    }
}

fn file_report_total_occurrences(clusters: &[ReportCluster]) -> usize {
    clusters
        .iter()
        .map(|cluster| cluster.occurrences.len())
        .sum()
}

/// The nearly-identical, two-file cluster the message tests describe.
fn two_file_cluster() -> ReportCluster {
    sample_cluster(
        "c",
        100.0,
        vec![occurrence("a.cs", 0, 1), occurrence("b.cs", 0, 1)],
        "nearly_identical",
    )
}

/// Diagnostics published for a single-cluster report rooted at `path`.
fn diagnostics_for(cluster: ReportCluster, path: &str, workspace: &Path) -> Vec<Diagnostic> {
    let total_occurrences = file_report_total_occurrences(std::slice::from_ref(&cluster));
    let file_report = FileReport {
        path: PathBuf::from(path),
        clusters: vec![cluster],
        total_occurrences,
    };
    build_for_file(&file_report, workspace)
}

/// Reads the machine-readable cluster id out of a diagnostic's `data` payload.
fn cluster_id_of(data: Option<&serde_json::Value>) -> Result<&str> {
    data.and_then(|payload| payload.get("cluster_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("cluster id stored in diagnostic data"))
}

/// The hover must carry exactly one "Canonical" link, never an occurrence dump.
fn assert_single_canonical_link(diagnostic: &Diagnostic, context: &str) -> Result<()> {
    let related = diagnostic
        .related_information
        .as_ref()
        .ok_or_else(|| anyhow!("related info for {context}"))?;
    assert_eq!(
        related.len(),
        1,
        "{context} must yield exactly 1 canonical link, not a full occurrence dump: {related:?}"
    );
    let canonical = related
        .first()
        .ok_or_else(|| anyhow!("canonical entry (len asserted above)"))?;
    assert_eq!(
        canonical.message, "Canonical",
        "related label must be 'Canonical', not an indexed occurrence string: {}",
        canonical.message
    );
    Ok(())
}

// [LSP-SEVERITY-BUCKET] Bucket → severity mapping.
#[test]
fn severity_for_maps_bucket_to_lsp_level() {
    for (bucket, expected_severity, rationale) in BUCKET_SEVERITIES {
        let cluster = sample_cluster(bucket, 1.0, vec![occurrence("a.cs", 0, 1)], bucket);
        assert_eq!(severity_for(&cluster), expected_severity, "{rationale}");
    }
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
    assert_eq!(cluster_id_of(Some(&diagnostic_data(&cluster)))?, "abc123");
    Ok(())
}

#[test]
fn diagnostic_message_shows_category_count_and_action() {
    let message = diagnostic_message(&two_file_cluster());
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

// [FUSION-CONTENT-GATE] #344: the bucket title alone cannot tell a
// corroborated Type-2 rename from an anchor-poor scaffolding family — both
// render structural=1.00. The diagnostic must state the fused confidence and
// the measured content evidence the gate scored, using the one shared
// `render::signals` rendering.
#[test]
fn diagnostic_message_states_fused_confidence_and_measured_content_evidence() {
    let cluster = two_file_cluster();
    let message = diagnostic_message(&cluster);
    assert!(
        message.contains("Nearly identical code × 2"),
        "the existing human label and count survive the addition: {message}"
    );
    for (evidence, axis) in SAMPLE_EVIDENCE_AXES {
        assert!(message.contains(evidence), "{axis}: {message}");
    }
    assert!(
        message.ends_with(&deslop_core::render::signals::plain_explanation(
            cluster.signals
        )),
        "the explanation must be the shared render::signals rendering, never a \
         second hand-rolled formatter: {message}"
    );
}

// A cluster with different evidence must produce a different message — pins
// that the text reads this cluster's signals, not a constant.
#[test]
fn diagnostic_message_tracks_each_clusters_own_evidence() {
    let mut anchor_poor = sample_cluster(
        "scaffolding",
        100.0,
        vec![occurrence("a.cs", 0, 1), occurrence("b.cs", 0, 1)],
        "structural_only",
    );
    anchor_poor.signals = ReportSignals {
        structural: 1.0,
        token_jaccard: 0.0,
        embedding_cos: 0.0,
        fused: 0.33,
        agreement: 0.04,
        rename_consistency: 0.02,
        literal_fraction: 0.77,
    };
    let message = diagnostic_message(&anchor_poor);
    assert!(
        message.contains("structural 1.00 · jaccard 0.00 · embedding 0.00"),
        "shape-only support: {message}"
    );
    assert!(
        message.contains("fused 0.33 · agreement 0.04 · rename 0.02 · literal 0.77"),
        "anchor-poor evidence is what separates this from a real rename: {message}"
    );
    assert!(
        !message.contains("agreement 0.58"),
        "must not echo another cluster's evidence: {message}"
    );
}

// [LSP-SEVERITY-BUCKET] Identical code → Error; canonical link present.
#[test]
fn build_for_file_emits_error_for_identical_cluster_with_canonical_link() -> Result<()> {
    let workspace = TempDir::new()?;
    let _primary = write_source(workspace.path(), "Alpha.cs", "alpha\nbeta\ngamma\n")?;
    let _secondary = write_source(workspace.path(), "Beta.cs", "a\nbb\nccc\ndddd\n")?;
    let occurrences = vec![occurrence("Alpha.cs", 0, 5), occurrence("Beta.cs", 2, 5)];
    let cluster = sample_cluster("cluster-1", 100.0, occurrences, "identical");
    let diagnostics = diagnostics_for(cluster, "Alpha.cs", workspace.path());
    assert_eq!(
        diagnostics.len(),
        1,
        "one diagnostic for the Alpha.cs occurrence"
    );
    let diagnostic = diagnostics
        .first()
        .ok_or_else(|| anyhow!("diagnostic present"))?;
    assert_eq!(diagnostic.source.as_deref(), Some("deslop"));
    // [FUSION-CONTENT-GATE] #344: the evidence reaches the published
    // Diagnostic, not merely the formatter.
    assert!(
        diagnostic
            .message
            .contains("fused 0.91 · agreement 0.58 · rename 0.72 · literal 0.24"),
        "published diagnostic carries the fused score and content evidence: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.starts_with("Identical code × 2 — "),
        "the existing bucket title and count are still first: {}",
        diagnostic.message
    );
    assert_eq!(
        diagnostic.severity,
        Some(DiagnosticSeverity::ERROR),
        "Identical bucket → Error per [LSP-SEVERITY-BUCKET]"
    );
    assert!(
        diagnostic.code.is_none(),
        "cluster hash must not be visible as deslop(<id>) in editor hovers"
    );
    assert_eq!(cluster_id_of(diagnostic.data.as_ref())?, "cluster-1");
    assert_single_canonical_link(diagnostic, "a two-occurrence identical cluster")?;
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
    for (bucket, expected_severity, rationale) in BUCKET_SEVERITIES {
        let cluster = sample_cluster("c", 1.0, vec![occurrence("A.cs", 0, 2)], bucket);
        let diagnostics = diagnostics_for(cluster, "A.cs", workspace.path());
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
            "bucket '{bucket}' → {expected_severity:?} ({rationale})"
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
    let diagnostics = diagnostics_for(cluster, "Alpha.cs", workspace.path());
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
    let _primary = write_source(workspace.path(), "Main.cs", "fn a() {}\n")?;
    let _other = write_source(workspace.path(), "Other.cs", "fn b() {}\n")?;
    let mut occs = vec![occurrence("Main.cs", 0, 5)];
    for _ in 0..37 {
        occs.push(occurrence("Other.cs", 0, 3));
    }
    let cluster = sample_cluster("big", 100.0, occs, "identical");
    let diagnostics = diagnostics_for(cluster, "Main.cs", workspace.path());
    let diagnostic = diagnostics.first().ok_or_else(|| anyhow!("diagnostic"))?;
    assert_single_canonical_link(diagnostic, "38 occurrences")
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
