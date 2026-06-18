use crate::support::*;

/// Creates a `tempdir` with a `src` scan root seeded with the canonical
/// clone pair, returning both so the `tempdir` guard stays alive.
fn clone_pair_scan_root() -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    Ok((tmp, scan_root))
}

/// Creates a `tempdir` with an empty `src` scan root, returning both so
/// the `tempdir` guard stays alive.
fn empty_scan_root() -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    Ok((tmp, scan_root))
}

/// Writes a `.deslop.toml` into `scan_root` whose `[threshold]` block
/// pins `max_duplication_percent` to `percent`.
fn write_threshold_config(scan_root: &Path, percent: &str) -> Result<()> {
    fs::write(
        scan_root.join(".deslop.toml"),
        format!("[threshold]\nmax_duplication_percent = {percent}\n"),
    )?;
    Ok(())
}

/// Runs `deslop --fail-over <value>` against an empty scan root and
/// asserts the run exits with clap's argument-error code 2.
fn assert_fail_over_value_exits_two(value: &str) -> Result<()> {
    let (tmp, scan_root) = empty_scan_root()?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--fail-over", value, "--no-color"])
        .assert()
        .code(2);
    Ok(())
}

#[test]
fn fail_over_cli_passes_under_threshold() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--fail-over", "100", "--no-color"])
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("cli"));
    assert_eq!(threshold_field(&json, "breached").as_bool(), Some(false));
    Ok(())
}

// Implements [EXIT-CODES]: the `[threshold]` key in `.deslop.toml` is
// loaded when `--fail-over` is absent, and an exceeded value exits 3.
#[test]
fn fail_over_config_file_loaded_when_flag_absent() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    write_threshold_config(&scan_root, "0.0")?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--no-color"])
        .assert()
        .code(3);
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("config"));
    assert_eq!(threshold_field(&json, "breached").as_bool(), Some(true));
    Ok(())
}

// Implements [EXIT-CODES]: `--fail-over` overrides the config-file key.
// A permissive CLI value turns a breaching config into a passing run.
#[test]
fn fail_over_cli_overrides_config_file() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    write_threshold_config(&scan_root, "0.0")?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--fail-over", "100", "--no-color"])
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("cli"));
    assert_eq!(threshold_field(&json, "breached").as_bool(), Some(false));
    Ok(())
}

// Implements [EXIT-CODES]: `--no-fail-over` clears the config threshold
// so the run is ungated locally.
#[test]
fn no_fail_over_overrides_config_file_threshold() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    write_threshold_config(&scan_root, "0.0")?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--no-fail-over", "--no-color"])
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    Ok(())
}

// Implements [EXIT-CODES]: invalid `--fail-over` values (negative, NaN,
// > 100) produce clap's argument-error exit code 2.
#[test]
fn fail_over_invalid_value_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two("-1.0")
}

// Implements [METRICS-REPO] + [OUTPUT-SCHEMA-JSON]: `--from-report`
// replays a v3 report, including its metrics block, without re-running
// the pipeline. Applied `--fail-over` on the replay beats any earlier
// threshold.
#[test]
fn from_report_replays_metrics_without_reanalysing() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    let initial = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--no-color"])
        .assert()
        .success();
    let original = read_json_report(&initial.json)?;
    let original_metrics = field(&original, "metrics").clone();
    // Replay: write into a second output prefix so we don't clobber
    // the source JSON, and re-render from the first.
    let replay_prefix = tmp.path().join("replay");
    let mut cmd2 = deslop_command(&scan_root, &replay_prefix)?;
    let _assertion2 = cmd2
        .arg("--from-report")
        .arg(&initial.json)
        .arg("--no-color")
        .assert()
        .success();
    let replay_json = read_json_report(&with_ext(&replay_prefix, "json"))?;
    assert_eq!(
        field(&replay_json, "metrics").clone(),
        original_metrics,
        "metrics must round-trip through --from-report"
    );
    Ok(())
}

// Implements [METRICS-REPO]: the text renderer prints the one-line
// repo duplication header.
#[test]
fn text_renderer_shows_repo_duplication_header() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--fail-over", "0", "--no-color"])
        .assert()
        .code(3);
    let txt = fs::read_to_string(&out.txt)?;
    assert!(
        txt.contains("repo:") && txt.contains("% duplicated"),
        "text renderer must print repo metric: {txt}"
    );
    assert!(
        txt.contains("threshold:") && txt.contains("breached"),
        "text renderer must print breach verdict: {txt}"
    );
    Ok(())
}

// Implements [METRICS-REPO]: the HTML renderer emits a banner whose
// CSS class reflects the threshold verdict — breached → red, ok →
// green, absent → neutral.
#[test]
fn html_renderer_colour_codes_threshold_state() -> Result<()> {
    // Breached variant.
    let (tmp, scan_root) = clone_pair_scan_root()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--fail-over", "0", "--no-color"])
        .assert()
        .code(3);
    let html_breached = fs::read_to_string(&out.html)?;
    assert!(
        html_breached.contains("metrics-banner--breached"),
        "breached HTML must carry the breached class"
    );

    // Neutral variant (no threshold).
    let (tmp2, scan_root2) = clone_pair_scan_root()?;
    let out2 = outputs_under(tmp2.path());
    let mut cmd2 = deslop_command(&scan_root2, &tmp2.path().join("report"))?;
    let _assertion2 = cmd2
        .args(["--min-nodes", "8", "--no-color"])
        .assert()
        .success();
    let html_neutral = fs::read_to_string(&out2.html)?;
    assert!(
        html_neutral.contains("metrics-banner--neutral"),
        "no-threshold HTML must carry the neutral class"
    );
    Ok(())
}

// Implements [EXIT-CODES]: `--fail-over 150` is out of range and exits 2.
#[test]
fn fail_over_above_100_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two("150.0")
}

// Implements [EXIT-CODES]: `--fail-over NaN` is not finite and exits 2.
#[test]
fn fail_over_nan_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two("NaN")
}

// Implements [EXIT-CODES]: an invalid threshold in `.deslop.toml`
// propagates as exit 1 (runtime error) with the offending path in the
// diagnostic. `max_duplication_percent = 150` is out of range.
#[test]
fn config_threshold_out_of_range_fails_runtime() -> Result<()> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    write_threshold_config(&scan_root, "150.0")?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--no-color"])
        .assert()
        .code(1);
    Ok(())
}
