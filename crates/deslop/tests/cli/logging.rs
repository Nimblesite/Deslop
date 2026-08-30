use super::support::*;

const RUST_LOG_ENV: &str = "RUST_LOG";
const LOG_TO_CONSOLE_FLAG: &str = "--log-to-console";
const LOG_LEVEL_FLAG: &str = "--log-level";
const TECHNICAL_FLAG: &str = "--technical";
const DESLOP_INVOKED_MESSAGE: &str = "deslop invoked";

/// Builds a `deslop` command against the `csharp-small` fixture writing
/// its report under `<tmp>/report`. Every logging test shares this scan
/// root + output layout; only the flag/env combination differs.
fn csharp_small_command(tmp: &tempfile::TempDir) -> Result<Command> {
    fixture_command(CSHARP_SMALL_FIXTURE, &tmp.path().join(REPORT_OUTPUT_STEM))
}

/// Decodes the captured stderr of a finished assertion into an owned
/// `String` so the test can assert on the rendered console output.
fn stderr_text(assertion: &assert_cmd::assert::Assert) -> Result<String> {
    Ok(std::str::from_utf8(&assertion.get_output().stderr)?.to_owned())
}

#[test]
fn default_run_writes_log_to_timestamped_file_not_stderr() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .env_remove(RUST_LOG_ENV)
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE, NO_COLOR_FLAG])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        !stderr.contains(" INFO "),
        "default stderr must not carry tracing INFO lines: {stderr}"
    );
    assert!(
        stderr.contains("Found"),
        "default stderr must carry the summary block: {stderr}"
    );
    assert!(
        stderr.contains("done"),
        "default stderr must carry the success footer: {stderr}"
    );
    assert!(
        out.json.exists(),
        "json still written: {}",
        out.json.display()
    );
    let log_files = find_timestamped_logs(tmp.path())?;
    assert_eq!(
        log_files.len(),
        1,
        "expected exactly one timestamped log file, found {log_files:?}",
    );
    let log_file = log_files
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("log_files vec unexpectedly empty"))?;
    let log_body = fs::read_to_string(&log_file)?;
    assert!(
        log_body.contains(DESLOP_INVOKED_MESSAGE),
        "log file missing the invoked event: {log_body}"
    );
    Ok(())
}

// Implements [UX-LOG-CONSOLE]: `--log-to-console` routes log events
// back to stderr instead of the file.
#[test]
fn log_to_console_flag_routes_events_to_stderr() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .env_remove(RUST_LOG_ENV)
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            LOG_TO_CONSOLE_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains(DESLOP_INVOKED_MESSAGE),
        "--log-to-console must surface the invoked event on stderr: {stderr}"
    );
    let log_files = find_timestamped_logs(tmp.path())?;
    assert!(
        log_files.is_empty(),
        "--log-to-console must not create a log file: {log_files:?}",
    );
    Ok(())
}

// Implements [UX-LOG-LEVEL]: `--log-level warn` suppresses INFO
// events. The canonical "deslop invoked" INFO message must not
// appear in the log file when the level is raised.
#[test]
fn log_level_warn_suppresses_info_events() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let _assertion = cmd
        .env_remove(RUST_LOG_ENV)
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            LOG_LEVEL_FLAG,
            "warn",
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let log_path = find_timestamped_logs(tmp.path())?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no timestamped log file written"))?;
    let log_body = fs::read_to_string(&log_path)?;
    assert!(
        !log_body.contains(DESLOP_INVOKED_MESSAGE),
        "warn level must suppress the INFO invoked event: {log_body}"
    );
    Ok(())
}

// Implements [UX-PREAMBLE]: the preamble line is emitted before the
// pipeline runs and names the scan path + output paths. `--technical`
// additionally surfaces the min-nodes / embeddings / incremental knobs.
#[test]
fn preamble_announces_what_the_run_will_do() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            TECHNICAL_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains("deslop scanning"),
        "preamble must announce the scan: {stderr}"
    );
    assert!(
        stderr.contains("min-nodes=8"),
        "--technical preamble must surface the min-nodes knob: {stderr}"
    );
    assert!(
        stderr.contains("report →"),
        "preamble must show where the report goes: {stderr}"
    );
    assert!(
        stderr.contains("log    →"),
        "preamble must show where the log goes: {stderr}"
    );
    Ok(())
}

// Implements [UX-NO-COLOR]: the `--no-color` flag suppresses ANSI
// escape sequences in the stderr output. Used by CI and by pipes.
#[test]
fn no_color_flag_suppresses_ansi_escapes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE, NO_COLOR_FLAG])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        !stderr.contains('\x1b'),
        "--no-color must strip ANSI escapes: {stderr:?}"
    );
    Ok(())
}

// Implements [UX-COLOR-FORCE]: `DESLOP_FORCE_COLOR=1` forces ANSI
// escapes even when stderr isn't a TTY (useful in CI logs). The flag
// combination also exercises the `ColorChoice::Always` branch in
// coverage.
#[test]
fn color_force_env_emits_ansi_escapes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .env("DESLOP_FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains('\x1b'),
        "DESLOP_FORCE_COLOR must emit ANSI escapes: {stderr:?}"
    );
    Ok(())
}

// Implements [UX-LOG-RUST-LOG]: `RUST_LOG` takes precedence over
// `--log-level` — Rust-ecosystem convention. Setting `RUST_LOG=warn`
// with `--log-to-console` must still produce the `deslop invoked`
// info message when we *also* set `--log-level info`, because the
// environment variable wins. Conversely, `RUST_LOG=warn` alone
// suppresses it.
#[test]
fn rust_log_env_controls_severity_filter() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .env(RUST_LOG_ENV, "warn")
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            LOG_TO_CONSOLE_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        !stderr.contains(DESLOP_INVOKED_MESSAGE),
        "RUST_LOG=warn must suppress INFO events: {stderr}"
    );
    Ok(())
}

// Implements [UX-COLOR-NO-COLOR-ENV]: `NO_COLOR=1` disables ANSI
// escapes even when `DESLOP_FORCE_COLOR` is also set — standard
// NO_COLOR precedence per <https://no-color.org>.
#[test]
fn no_color_env_overrides_force_color() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .env("NO_COLOR", "1")
        .env("DESLOP_FORCE_COLOR", "1")
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        !stderr.contains('\x1b'),
        "NO_COLOR must override the force flag: {stderr:?}"
    );
    Ok(())
}

// Implements [UX-TECHNICAL-CACHE]: `--technical --incremental`
// surfaces the raw `cache: N hit / M miss` line on stderr. Plain
// mode only shows the friendly `skipped N unchanged file(s)` line
// — the technical branch lives under `if technical` in
// `write_cache_line` and is otherwise unreachable.
#[test]
fn technical_mode_surfaces_raw_cache_stats_line() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture(CSHARP_SMALL_FIXTURE), &scan_root)?;
    // First run populates the cache.
    let mut first = deslop_command(&scan_root, &tmp.path().join("first"))?;
    let _assertion = first
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE])
        .assert()
        .success();
    let mut second = deslop_command(&scan_root, &tmp.path().join("second"))?;
    let assertion = second
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            TECHNICAL_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains("cache: 2 hit / 0 miss"),
        "--technical must surface the raw cache-stats line: {stderr}"
    );
    Ok(())
}

// Implements [UX-TECHNICAL-EMBEDDINGS]: `--technical` with a live
// embedding provider prints the provenance triple
// `provider/model@version (N-d)` on stderr. [REMOVE-STUB] Uses a mock
// Ollama HTTP server so the test exercises the production ollama
// provider end-to-end without depending on a real install.
#[test]
fn technical_mode_surfaces_embedding_provenance_line() -> Result<()> {
    let server = crate::mock_ollama::MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture(CSHARP_SMALL_FIXTURE), &scan_root)?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let assertion = cmd
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
        ])
        .arg(server.endpoint())
        .args([TECHNICAL_FLAG, NO_COLOR_FLAG])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains("embeddings: ollama/nomic-embed-text@"),
        "--technical must surface the provenance triple on stderr: {stderr}"
    );
    Ok(())
}

// Implements [UX-TECHNICAL-BREAKDOWN]: `--technical` prints the
// researcher breakdown row with bracketed taxonomy labels. Plain mode
// uses friendly wording; this test guards the taxonomy string the
// technical branch emits. The csharp-small pair is a maximal Type-2
// rename with every literal preserved, so [FUSED-CONTENT-GATE] rename
// consistency routes it to the act-now `nearly_identical` bucket's
// hybrid title ([CLONE-BUCKETS-DUAL-LABEL]).
#[test]
fn technical_mode_uses_type_taxonomy_in_breakdown_row() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = csharp_small_command(&tmp)?;
    let assertion = cmd
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            TECHNICAL_FLAG,
            NO_COLOR_FLAG,
        ])
        .assert()
        .success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        stderr.contains("1 × Nearly identical code [Type-3]"),
        "--technical must print the bracketed-taxonomy breakdown: {stderr}"
    );
    assert!(
        stderr.contains("#1  ● Nearly identical code [Type-3]"),
        "--technical must print the bracketed taxonomy in the ranked row: {stderr}"
    );
    Ok(())
}

// Implements [UX-PLAIN-SUMMARY]: empty scan root (no source files)
// produces a report with zero clusters, which the plain-mode
// summary must render without panicking or emitting the
// "Worst offender" callout.
#[test]
fn plain_summary_on_empty_scan_root_has_no_worst_offender_line() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty)?;
    let mut cmd = deslop_command(&empty, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let assertion = cmd.arg(NO_COLOR_FLAG).assert().success();
    let stderr = stderr_text(&assertion)?;
    assert!(
        !stderr.contains("Worst offender"),
        "empty scan must not print a worst-offender line: {stderr}"
    );
    Ok(())
}
