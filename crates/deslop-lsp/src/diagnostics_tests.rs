use super::*;
use anyhow::{anyhow, Result};
use deslop_core::report::ReportCluster;
use tempfile::TempDir;

const ALPHA_FILE: &str = "Alpha.cs";
const A_CAPITAL_FILE: &str = "A.cs";
const MAIN_FILE: &str = "Main.cs";
const A_FILE: &str = "a.cs";
const HELLO_SOURCE: &str = "hello\n";
const LIGHT_CLUSTER_MASS: u64 = 1;
const HEAVY_CLUSTER_MASS: u64 = 100;
const FIXTURE_END_BYTE: usize = 5;

// [LSP-SEVERITY-BAND] Every mass rank band, the severity it must publish,
// and the rationale that mapping pins. Severity is a function of the
// mass-derived rank band, never of pair measurements.
const RANK_BAND_SEVERITIES: [(&str, DiagnosticSeverity, &str); 4] = [
    (
        "worst",
        DiagnosticSeverity::ERROR,
        "Worst band → Error (highest duplicated mass in the report)",
    ),
    (
        "top10",
        DiagnosticSeverity::WARNING,
        "Top-10 band → Warning",
    ),
    (
        "mid",
        DiagnosticSeverity::INFORMATION,
        "Mid band → Information",
    ),
    ("faint", DiagnosticSeverity::HINT, "Tail band → Hint"),
];

fn write_source(dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, body)?;
    Ok(path)
}

/// Mass-only fixture cluster: id, rank band, mass, canonical extent, and
/// membership. No signals, no bucket, no weight — the cluster carries
/// membership and mass only ([FUSED-PAIR-SIGNALS]).
fn sample_cluster(id: &str, mass: u64, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
    // restamp_fixture recomputes mass as canonical_node_count × (count − 1),
    // so the canonical extent is derived from the requested mass and the
    // cluster's membership ([RANK-MASS-SUM]).
    let visible = occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count();
    let mut cluster = deslop_core::report_fixtures::fixture_cluster(id, occurrences);
    let copies = u64::try_from(visible.saturating_sub(1)).unwrap_or(0).max(1);
    let canonical_node_count = mass
        .checked_div(copies)
        .and_then(|nodes| usize::try_from(nodes).ok());
    assert!(
        canonical_node_count.is_some(),
        "fixture mass must be divisible by its visible-copy count and fit usize"
    );
    cluster.canonical_node_count = canonical_node_count.unwrap_or_default();
    cluster.mass = mass;
    deslop_core::report_fixtures::restamp_fixture(&mut cluster);
    cluster
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

/// The two-file cluster the message tests describe.
fn two_file_cluster() -> ReportCluster {
    sample_cluster(
        "c",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(A_FILE, 0, 1), occurrence("b.cs", 0, 1)],
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

// [LSP-SEVERITY-BAND] Rank band → severity mapping.
#[test]
fn severity_for_maps_rank_band_to_lsp_level() {
    for (band, expected_severity, rationale) in RANK_BAND_SEVERITIES {
        let mut cluster = sample_cluster(band, LIGHT_CLUSTER_MASS, vec![occurrence(A_FILE, 0, 1)]);
        cluster.rank_band = band.to_owned();
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
fn diagnostic_data_stores_cluster_id_and_mass_for_machine_readers() -> Result<()> {
    let cluster = sample_cluster(
        "abc123",
        LIGHT_CLUSTER_MASS,
        vec![
            occurrence(ALPHA_FILE, 0, FIXTURE_END_BYTE),
            occurrence("b.cs", 0, FIXTURE_END_BYTE),
        ],
    );
    let data = diagnostic_data(&cluster);
    assert_eq!(cluster_id_of(Some(&data))?, "abc123");
    assert_eq!(
        data.get("mass").and_then(serde_json::Value::as_u64),
        Some(LIGHT_CLUSTER_MASS)
    );
    assert_eq!(
        data.get("rank_band").and_then(serde_json::Value::as_str),
        Some("worst")
    );
    Ok(())
}

#[test]
fn diagnostic_message_shows_count_and_mass() {
    let message = diagnostic_message(&two_file_cluster());
    assert!(message.contains(" — "), "joined with em dash: {message}");
    assert!(
        message.starts_with("Duplicate code × 2"),
        "neutral title and instance count first: {message}"
    );
    assert!(
        message.contains(&format!("mass {HEAVY_CLUSTER_MASS}")),
        "message carries the duplicated mass: {message}"
    );
    assert!(
        !message.contains("Type-"),
        "diagnostic message must not expose clone taxonomy labels: {message}"
    );
}

// [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
// never touch the cluster. An LSP diagnostic on one occurrence must not
// render them: the message quotes the neutral count and the duplicated
// mass, and nothing else.
#[test]
fn diagnostic_message_renders_no_pair_evidence() {
    let cluster = two_file_cluster();
    let message = diagnostic_message(&cluster);
    assert!(
        message.contains("Duplicate code × 2"),
        "the neutral label and count survive: {message}"
    );
    assert!(
        !message.contains("fused"),
        "no cluster fused score on any surface: {message}"
    );
    for axis in [
        "structural",
        "jaccard",
        "embedding",
        "agreement",
        "rename",
        "literal",
    ] {
        assert!(
            !message.contains(axis),
            "pair evidence must not reach the diagnostic ({axis}): {message}"
        );
    }
    assert!(
        !message.contains("measured pair") && !message.contains("occurrences 1 and 2"),
        "no pair attribution on a cluster surface: {message}"
    );
}

// The message is a pure function of count and duplicated mass: two
// clusters with the same membership shape and mass quote the same text
// regardless of any pair measurements, and a mass difference shows.
#[test]
fn diagnostic_message_depends_on_count_and_mass_only() {
    let same_mass = sample_cluster(
        "twin",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(A_FILE, 0, 1), occurrence("b.cs", 0, 1)],
    );
    assert_eq!(
        diagnostic_message(&same_mass),
        diagnostic_message(&two_file_cluster()),
        "same count and mass → same message: {}",
        diagnostic_message(&same_mass)
    );
    let heavier = sample_cluster(
        "heavy",
        HEAVY_CLUSTER_MASS * 2,
        vec![occurrence(A_FILE, 0, 1), occurrence("b.cs", 0, 1)],
    );
    assert_ne!(
        diagnostic_message(&heavier),
        diagnostic_message(&two_file_cluster()),
        "mass must show in the message: {} vs {}",
        diagnostic_message(&heavier),
        diagnostic_message(&two_file_cluster())
    );
}

#[test]
fn diagnostic_never_renders_pair_scores() {
    let cluster = sample_cluster(
        "unsourced",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(A_FILE, 0, 1)],
    );
    let message = diagnostic_message(&cluster);
    assert!(message.contains("Duplicate code × 1"));
    assert!(
        !message.contains("structural"),
        "unsourced structural score leaked: {message}"
    );
    assert!(
        !message.contains("agreement"),
        "unsourced content score leaked: {message}"
    );
}

// [LSP-SEVERITY-BAND] Worst band → Error; canonical link present.
#[test]
fn build_for_file_emits_error_for_worst_band_cluster_with_canonical_link() -> Result<()> {
    let workspace = TempDir::new()?;
    let _primary = write_source(workspace.path(), ALPHA_FILE, "alpha\nbeta\ngamma\n")?;
    let _secondary = write_source(workspace.path(), "Beta.cs", "a\nbb\nccc\ndddd\n")?;
    let occurrences = vec![
        occurrence(ALPHA_FILE, 0, FIXTURE_END_BYTE),
        occurrence("Beta.cs", 2, FIXTURE_END_BYTE),
    ];
    let cluster = sample_cluster("cluster-1", HEAVY_CLUSTER_MASS, occurrences);
    let diagnostics = diagnostics_for(cluster, ALPHA_FILE, workspace.path());
    assert_eq!(
        diagnostics.len(),
        1,
        "one diagnostic for the Alpha.cs occurrence"
    );
    let diagnostic = diagnostics
        .first()
        .ok_or_else(|| anyhow!("diagnostic present"))?;
    assert_eq!(diagnostic.source.as_deref(), Some("deslop"));
    // [FUSED-PAIR-SIGNALS] The diagnostic is a cluster surface and renders
    // no pair signals.
    assert!(
        !diagnostic.message.contains("agreement"),
        "published diagnostic must not carry content evidence: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.starts_with("Duplicate code × 2 — "),
        "the neutral title and count are still first: {}",
        diagnostic.message
    );
    assert_eq!(
        diagnostic.severity,
        Some(DiagnosticSeverity::ERROR),
        "worst rank band → Error per [LSP-SEVERITY-BAND]"
    );
    assert!(
        diagnostic.code.is_none(),
        "cluster hash must not be visible as deslop(<id>) in editor hovers"
    );
    assert_eq!(cluster_id_of(diagnostic.data.as_ref())?, "cluster-1");
    assert_single_canonical_link(diagnostic, "a two-occurrence cluster")?;
    assert_eq!(
        diagnostic.range.start.line, 0,
        "start on first line of Alpha.cs"
    );
    Ok(())
}

// [LSP-SEVERITY-BAND] All rank bands publish diagnostics — none are suppressed by default.
#[test]
fn build_for_file_publishes_all_rank_bands_with_correct_severity() -> Result<()> {
    let workspace = TempDir::new()?;
    let _primary = write_source(workspace.path(), A_CAPITAL_FILE, "abc\n")?;
    for (band, expected_severity, rationale) in RANK_BAND_SEVERITIES {
        let mut cluster = sample_cluster(
            "c",
            LIGHT_CLUSTER_MASS,
            vec![occurrence(A_CAPITAL_FILE, 0, 2)],
        );
        cluster.rank_band = band.to_owned();
        let diagnostics = diagnostics_for(cluster, A_CAPITAL_FILE, workspace.path());
        assert_eq!(
            diagnostics.len(),
            1,
            "band '{band}' must always produce a diagnostic (no mass-percentile suppression)"
        );
        let diag = diagnostics
            .first()
            .ok_or_else(|| anyhow!("no diagnostic for band '{band}'"))?;
        assert_eq!(
            diag.severity,
            Some(expected_severity),
            "band '{band}' → {expected_severity:?} ({rationale})"
        );
    }
    Ok(())
}

#[test]
fn build_for_file_empty_related_info_becomes_none() -> Result<()> {
    let workspace = TempDir::new()?;
    let _primary = write_source(workspace.path(), ALPHA_FILE, "abcdef\n")?;
    let cluster = sample_cluster(
        "solo",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(ALPHA_FILE, 0, 3)],
    );
    let diagnostics = diagnostics_for(cluster, ALPHA_FILE, workspace.path());
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
    let _primary = write_source(workspace.path(), MAIN_FILE, "fn a() {}\n")?;
    let _other = write_source(workspace.path(), "Other.cs", "fn b() {}\n")?;
    let mut occs = vec![occurrence(MAIN_FILE, 0, FIXTURE_END_BYTE)];
    for _ in 0..37 {
        occs.push(occurrence("Other.cs", 0, 3));
    }
    let cluster = sample_cluster("big", HEAVY_CLUSTER_MASS, occs);
    let diagnostics = diagnostics_for(cluster, MAIN_FILE, workspace.path());
    let diagnostic = diagnostics.first().ok_or_else(|| anyhow!("diagnostic"))?;
    assert_single_canonical_link(diagnostic, "38 occurrences")
}

#[test]
fn load_cached_source_reuses_cache_and_survives_missing_files() -> Result<()> {
    let workspace = TempDir::new()?;
    let real = write_source(workspace.path(), "Real.cs", HELLO_SOURCE)?;
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let first = load_cached_source(&real, &mut cache);
    assert_eq!(first, HELLO_SOURCE);
    assert!(cache.contains_key(&real), "entry cached after first read");
    let missing = workspace.path().join("missing.cs");
    let body = load_cached_source(&missing, &mut cache);
    assert!(
        body.is_empty(),
        "missing files fall back to empty string, not panic"
    );
    let second = load_cached_source(&real, &mut cache);
    assert_eq!(second, HELLO_SOURCE, "cached read returns same content");
    Ok(())
}
