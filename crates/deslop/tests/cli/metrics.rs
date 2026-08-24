use super::support::*;

/// Writes the cross-file clone-pair fixture into a fresh temp dir, runs
/// `deslop` with the given `--min-nodes` value, asserts the run
/// succeeded, and returns the fixture's analysed-line count alongside the
/// parsed JSON report. The temp dir is dropped on return — the report is
/// already fully materialised in the returned `Value`.
fn run_clone_pair(min_nodes: &str) -> Result<(u64, Value)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let analysed = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", min_nodes]).assert().success();
    let json = read_json_report(&out.json)?;
    Ok((analysed, json))
}

#[test]
fn metrics_zero_on_empty_corpus() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("empty");
    fs::create_dir_all(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.assert().success();
    let json = read_json_report(&out.json)?;
    assert_eq!(metric_field(&json, "analysed_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "clusters_total").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_files").as_u64(), Some(0));
    let pct = metric_field(&json, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!((0.0..=0.0001).contains(&pct), "percent must be 0: {pct}");
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    assert!(
        metric_field(&json, "per_file")
            .as_array()
            .is_some_and(Vec::is_empty),
        "empty corpus yields no per_file rows"
    );
    Ok(())
}

// Implements [METRICS-REPO]: the `per_file` breakdown sums back to the
// repo aggregate, carries each file's own exact percentage, and arrives
// sorted worst-first on the wire.
#[test]
fn metrics_per_file_breakdown_matches_repo_totals() -> Result<()> {
    let (analysed, json) = run_clone_pair("8")?;
    let per_file = metric_field(&json, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        per_file.len(),
        2,
        "both fixture files must appear in per_file: {per_file:?}"
    );
    let analysed_sum: u64 = per_file
        .iter()
        .map(|entry| field(entry, "analysed_loc").as_u64().unwrap_or(0))
        .sum();
    assert_eq!(
        analysed_sum, analysed,
        "per_file analysed_loc must sum to the repo total: {per_file:?}"
    );
    let repo_dup = metric_field(&json, "duplicated_loc").as_u64().unwrap_or(0);
    let dup_sum: u64 = per_file
        .iter()
        .map(|entry| field(entry, "duplicated_loc").as_u64().unwrap_or(0))
        .sum();
    assert_eq!(
        dup_sum, repo_dup,
        "per_file duplicated_loc must sum to the repo total: {per_file:?}"
    );
    for entry in &per_file {
        assert_per_file_entry(entry);
    }
    let first = field(
        per_file.first().unwrap_or(&Value::Null),
        "duplication_percent",
    )
    .as_f64()
    .unwrap_or(0.0);
    let last = field(
        per_file.last().unwrap_or(&Value::Null),
        "duplication_percent",
    )
    .as_f64()
    .unwrap_or(0.0);
    assert!(
        first >= last,
        "per_file must be sorted worst-first: {per_file:?}"
    );
    Ok(())
}

/// Issue #286: `per_file[].path` is documented as relative to the scan
/// root — the same contract every occurrence path already honours — but
/// was emitted absolute. That inflates every JSON report by the length of
/// the scan root times the file count, and writes the user's home
/// directory into a file agents copy between machines.
#[test]
fn metrics_per_file_paths_are_scan_root_relative() -> Result<()> {
    let (_analysed, json) = run_clone_pair("8")?;
    let mut paths: Vec<String> = metric_field(&json, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| field(entry, "path").as_str().map(str::to_owned))
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        vec!["Alpha.cs".to_owned(), "Beta.cs".to_owned()],
        "per_file paths must be scan-root-relative, exactly as occurrence paths are"
    );
    Ok(())
}

/// Asserts one `per_file` row: a `.cs` path, non-zero duplication for
/// this cross-file clone fixture, and a percentage computed against the
/// file's own analysed-line denominator.
fn assert_per_file_entry(entry: &Value) {
    let dup = field(entry, "duplicated_loc").as_u64().unwrap_or(0);
    let file_analysed = field(entry, "analysed_loc").as_u64().unwrap_or(0);
    assert!(dup > 0, "both clone files must duplicate: {entry:?}");
    assert!(
        field(entry, "path")
            .as_str()
            .is_some_and(|path| path.contains(".cs")),
        "every FileMetric carries a source path: {entry:?}"
    );
    let dup32 = u32::try_from(dup).unwrap_or(u32::MAX);
    let analysed32 = u32::try_from(file_analysed).unwrap_or(u32::MAX);
    let expected = if analysed32 == 0 {
        0.0
    } else {
        f64::from(dup32) / f64::from(analysed32) * 100.0
    };
    let pct = field(entry, "duplication_percent").as_f64().unwrap_or(-1.0);
    assert!(
        (pct - expected).abs() < 0.01,
        "per-file percent uses the file's own denominator: got {pct}, want {expected}: {entry:?}"
    );
}

// Implements [METRICS-REPO]: duplicated_loc on a hand-counted fixture
// matches the lines covered by at least two non-hidden occurrences.
#[test]
fn metrics_match_hand_counted_fixture() -> Result<()> {
    let (analysed, json) = run_clone_pair("8")?;
    let metrics = field(&json, "metrics").clone();
    assert_eq!(
        metric_field(&json, "analysed_loc").as_u64(),
        Some(analysed),
        "analysed_loc mismatch: {metrics}"
    );
    let dup = metric_field(&json, "duplicated_loc").as_u64().unwrap_or(0);
    assert!(dup > 0, "duplicated_loc must exceed zero: {metrics}");
    assert!(
        dup <= analysed,
        "duplicated_loc {dup} cannot exceed analysed {analysed}",
    );
    let dup_files = metric_field(&json, "duplicated_files")
        .as_u64()
        .unwrap_or(0);
    assert!(
        dup_files >= 2,
        "both fixture files should contribute: {metrics}"
    );
    let clusters = field(&metrics, "clusters_total").as_u64().unwrap_or(0);
    assert!(clusters >= 1, "at least one cluster expected: {metrics}");
    Ok(())
}

// Implements [METRICS-REPO]: hidden occurrences (report_hide) do not
// count toward duplicated_loc. Hiding one of a two-file cross-file
// clone pair must shrink the metric and drop the hidden file from
// `duplicated_files`.
#[test]
fn metrics_exclude_hidden_occurrences() -> Result<()> {
    // Baseline without any hide policy.
    let tmp_plain = tempfile::tempdir()?;
    let plain_root = tmp_plain.path().join("src");
    let _ = write_clone_pair(&plain_root)?;
    let plain_out = outputs_under(tmp_plain.path());
    let mut cmd_plain = deslop_command(&plain_root, &tmp_plain.path().join("report"))?;
    let _plain_assertion = cmd_plain.args(["--min-nodes", "8"]).assert().success();
    let plain_metrics = field(&read_json_report(&plain_out.json)?, "metrics").clone();
    let plain_dup = field(&plain_metrics, "duplicated_loc")
        .as_u64()
        .unwrap_or(0);
    let plain_files = field(&plain_metrics, "duplicated_files")
        .as_u64()
        .unwrap_or(0);
    assert!(
        plain_dup > 0 && plain_files >= 2,
        "baseline must cover both files: {plain_metrics}"
    );

    // With Alpha.cs report_hidden: metric shrinks, hidden file drops
    // out of `duplicated_files`.
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[defaults]\nreport_hide = [\"**/Alpha.cs\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let metrics = field(&read_json_report(&out.json)?, "metrics").clone();
    let hidden_dup = field(&metrics, "duplicated_loc").as_u64().unwrap_or(0);
    let hidden_files = field(&metrics, "duplicated_files").as_u64().unwrap_or(0);
    assert!(
        hidden_dup < plain_dup,
        "hiding Alpha.cs must shrink duplicated_loc: plain={plain_dup} hidden={hidden_dup}: {metrics}",
    );
    assert!(
        hidden_files <= 1,
        "hidden files must not appear in duplicated_files: {metrics}"
    );
    Ok(())
}

// Implements [METRICS-REPO]: overlapping sibling-extension ranges count
// once per line. Two files with two clone pairs at different sizes must
// produce duplicated_loc <= lines in the files, never 2x that.
#[test]
fn metrics_deduplicate_overlapping_sibling_ranges() -> Result<()> {
    let (analysed, json) = run_clone_pair("4")?;
    let metrics = field(&json, "metrics").clone();
    let dup = field(&metrics, "duplicated_loc").as_u64().unwrap_or(0);
    assert!(
        dup <= analysed,
        "duplicated_loc {dup} must never exceed analysed {analysed} — \
         sibling-extension windows must be deduplicated per file: {metrics}"
    );
    Ok(())
}

// Implements [EXIT-CODES]: --fail-over 0.0 is breached by any
// duplication and the CLI exits 3 with the report on disk.
#[test]
fn fail_over_cli_exits_three_on_breach() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let assertion = cmd
        .args(["--min-nodes", "8", "--fail-over", "0", "--no-color"])
        .assert()
        .code(3);
    let _ = assertion;
    assert!(
        out.json.exists(),
        "report must land on disk before exit 3: {}",
        out.json.display()
    );
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("cli"));
    assert_eq!(threshold_field(&json, "breached").as_bool(), Some(true));
    Ok(())
}
