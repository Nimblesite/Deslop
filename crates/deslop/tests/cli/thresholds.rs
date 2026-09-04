use super::support::*;

const FAIL_OVER_FLAG: &str = "--fail-over";
const ZERO_THRESHOLD: &str = "0.0";
const SOURCE_FIELD: &str = "source";
const BREACHED_FIELD: &str = "breached";
const CLI_SOURCE: &str = "cli";
const CONFIG_SOURCE: &str = "config";
const THRESHOLD_BREACH_EXIT_CODE: i32 = 3;
/// clap's exit code for an argument the parser rejects.
const USAGE_EXIT_CODE: i32 = 2;
/// The exit code a runtime (non-argument) failure produces.
const RUNTIME_ERROR_EXIT_CODE: i32 = 1;
/// A `--fail-over` value no run can breach.
const PERMISSIVE_THRESHOLD: &str = "100";
/// A `--fail-over` value every duplicated run breaches.
const BREACHING_THRESHOLD: &str = "0";
/// A `--fail-over` value above the legal range.
const OUT_OF_RANGE_THRESHOLD: &str = "150.0";
/// A negative `--fail-over` value.
const NEGATIVE_THRESHOLD: &str = "-1.0";
/// A non-finite `--fail-over` value.
const NAN_THRESHOLD: &str = "NaN";
/// The `source` a report records when nothing gated the run.
const NO_SOURCE: &str = "none";
/// Flag clearing any configured threshold for this run.
const NO_FAIL_OVER_FLAG: &str = "--no-fail-over";
/// Output stem for the `--from-report` replay, kept apart from the
/// original so the replay cannot clobber its own source.
const REPLAY_OUTPUT_STEM: &str = "replay";

/// One prepared threshold scenario: the scan root, the paths the run
/// writes, and the command aimed at both.
struct ThresholdRun {
    /// Temp dir holding the scan root. Bound so it outlives the run —
    /// dropping it deletes the tree the command reads.
    tmp: TempDir,
    /// The `src` scan root seeded with the canonical clone pair.
    scan_root: PathBuf,
    /// The three report paths the run writes.
    out: RunOutputs,
    /// The `deslop` command, before its arguments are added.
    cmd: Command,
}

/// Opens a threshold scenario over the canonical clone pair.
fn clone_pair_run() -> Result<ThresholdRun> {
    let (tmp, scan_root) = clone_pair_scan_root()?;
    let out = outputs_under(tmp.path());
    let cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    Ok(ThresholdRun {
        tmp,
        scan_root,
        out,
        cmd,
    })
}

/// The argument list for a run gated at `percent`.
fn fail_over_args(percent: &str) -> [&str; 5] {
    [
        MIN_NODES_FLAG,
        MIN_NODES_VALUE,
        FAIL_OVER_FLAG,
        percent,
        NO_COLOR_FLAG,
    ]
}

/// The argument list for a run carrying no `--fail-over` flag.
const UNGATED_ARGS: [&str; 3] = [MIN_NODES_FLAG, MIN_NODES_VALUE, NO_COLOR_FLAG];

/// Asserts the report's `threshold` block names `source`.
fn assert_threshold_source(json: &Value, source: &str) {
    assert_eq!(threshold_field(json, SOURCE_FIELD).as_str(), Some(source));
}

/// Asserts the report's `threshold` block names `source` and records
/// `breached`.
fn assert_threshold(json: &Value, source: &str, breached: bool) {
    assert_threshold_source(json, source);
    assert_eq!(
        threshold_field(json, BREACHED_FIELD).as_bool(),
        Some(breached)
    );
}

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
    let mut cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let _assertion = cmd
        .args([FAIL_OVER_FLAG, value, NO_COLOR_FLAG])
        .assert()
        .code(USAGE_EXIT_CODE);
    Ok(())
}

#[test]
fn fail_over_cli_passes_under_threshold() -> Result<()> {
    let mut run = clone_pair_run()?;
    let _assertion = run
        .cmd
        .args(fail_over_args(PERMISSIVE_THRESHOLD))
        .assert()
        .success();
    assert_threshold(&read_json_report(&run.out.json)?, CLI_SOURCE, false);
    Ok(())
}

// Implements [EXIT-CODES]: the `[threshold]` key in `.deslop.toml` is
// loaded when `--fail-over` is absent, and an exceeded value exits 3.
#[test]
fn fail_over_config_file_loaded_when_flag_absent() -> Result<()> {
    let mut run = clone_pair_run()?;
    write_threshold_config(&run.scan_root, ZERO_THRESHOLD)?;
    let _assertion = run
        .cmd
        .args(UNGATED_ARGS)
        .assert()
        .code(THRESHOLD_BREACH_EXIT_CODE);
    assert_threshold(&read_json_report(&run.out.json)?, CONFIG_SOURCE, true);
    Ok(())
}

// Implements [EXIT-CODES]: `--fail-over` overrides the config-file key.
// A permissive CLI value turns a breaching config into a passing run.
#[test]
fn fail_over_cli_overrides_config_file() -> Result<()> {
    let mut run = clone_pair_run()?;
    write_threshold_config(&run.scan_root, ZERO_THRESHOLD)?;
    let _assertion = run
        .cmd
        .args(fail_over_args(PERMISSIVE_THRESHOLD))
        .assert()
        .success();
    assert_threshold(&read_json_report(&run.out.json)?, CLI_SOURCE, false);
    Ok(())
}

// Implements [EXIT-CODES]: `--no-fail-over` clears the config threshold
// so the run is ungated locally.
#[test]
fn no_fail_over_overrides_config_file_threshold() -> Result<()> {
    let mut run = clone_pair_run()?;
    write_threshold_config(&run.scan_root, ZERO_THRESHOLD)?;
    let _assertion = run
        .cmd
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            NO_FAIL_OVER_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    assert_threshold_source(&read_json_report(&run.out.json)?, NO_SOURCE);
    Ok(())
}

// Implements [EXIT-CODES]: invalid `--fail-over` values (negative, NaN,
// > 100) produce clap's argument-error exit code 2.
#[test]
fn fail_over_invalid_value_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two(NEGATIVE_THRESHOLD)
}

// Implements [METRICS-REPO] + [OUTPUT-SCHEMA-JSON]: `--from-report`
// replays a v3 report, including its metrics block, without re-running
// the pipeline. Applied `--fail-over` on the replay beats any earlier
// threshold.
#[test]
fn from_report_replays_metrics_without_reanalysing() -> Result<()> {
    let mut run = clone_pair_run()?;
    let _assertion = run.cmd.args(UNGATED_ARGS).assert().success();
    let original = read_json_report(&run.out.json)?;
    let original_metrics = field(&original, "metrics").clone();
    // Replay: write into a second output prefix so we don't clobber
    // the source JSON, and re-render from the first.
    let replay_prefix = run.tmp.path().join(REPLAY_OUTPUT_STEM);
    let mut cmd2 = deslop_command(&run.scan_root, &replay_prefix)?;
    let _assertion2 = cmd2
        .arg("--from-report")
        .arg(&run.out.json)
        .arg(NO_COLOR_FLAG)
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
    let mut run = clone_pair_run()?;
    let _assertion = run
        .cmd
        .args(fail_over_args(BREACHING_THRESHOLD))
        .assert()
        .code(THRESHOLD_BREACH_EXIT_CODE);
    let txt = fs::read_to_string(&run.out.txt)?;
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
    let mut breached = clone_pair_run()?;
    let _assertion = breached
        .cmd
        .args(fail_over_args(BREACHING_THRESHOLD))
        .assert()
        .code(THRESHOLD_BREACH_EXIT_CODE);
    let html_breached = fs::read_to_string(&breached.out.html)?;
    assert!(
        html_breached.contains("metrics-banner--breached"),
        "breached HTML must carry the breached class"
    );

    // Neutral variant (no threshold).
    let mut neutral = clone_pair_run()?;
    let _assertion2 = neutral.cmd.args(UNGATED_ARGS).assert().success();
    let html_neutral = fs::read_to_string(&neutral.out.html)?;
    assert!(
        html_neutral.contains("metrics-banner--neutral"),
        "no-threshold HTML must carry the neutral class"
    );
    Ok(())
}

// Implements [EXIT-CODES]: `--fail-over 150` is out of range and exits 2.
#[test]
fn fail_over_above_100_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two(OUT_OF_RANGE_THRESHOLD)
}

// Implements [EXIT-CODES]: `--fail-over NaN` is not finite and exits 2.
#[test]
fn fail_over_nan_exits_two() -> Result<()> {
    assert_fail_over_value_exits_two(NAN_THRESHOLD)
}

// Implements [EXIT-CODES]: an invalid threshold in `.deslop.toml`
// propagates as exit 1 (runtime error) with the offending path in the
// diagnostic. `max_duplication_percent = 150` is out of range.
#[test]
fn config_threshold_out_of_range_fails_runtime() -> Result<()> {
    let mut run = clone_pair_run()?;
    write_threshold_config(&run.scan_root, OUT_OF_RANGE_THRESHOLD)?;
    let _assertion = run
        .cmd
        .args(UNGATED_ARGS)
        .assert()
        .code(RUNTIME_ERROR_EXIT_CODE);
    Ok(())
}
