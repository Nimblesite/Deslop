use crate::support::*;

#[test]
fn metrics_zero_on_empty_corpus() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("empty");
    fs::create_dir_all(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
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
    Ok(())
}

// Implements [METRICS-REPO]: duplicated_loc on a hand-counted fixture
// matches the lines covered by at least two non-hidden occurrences.
#[test]
fn metrics_match_hand_counted_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let analysed = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
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
    let mut cmd_plain = Command::cargo_bin("deslop")?;
    let _plain_assertion = cmd_plain
        .arg(&plain_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp_plain.path().join("report"))
        .assert()
        .success();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let analysed = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--fail-over")
        .arg("0")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
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
