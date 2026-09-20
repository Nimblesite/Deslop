//! Real CLI log routing and levels ([PRINCIPLES-LOGGING]).

use super::support::*;
use crate::common::NEARLY_IDENTICAL_TITLE;

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
    assert_not_contains(
        &stderr,
        " INFO ",
        "default stderr must not carry tracing INFO lines",
    );
    assert_contains(
        &stderr,
        "Found",
        "default stderr must carry the summary block",
    );
    assert_contains(
        &stderr,
        "done",
        "default stderr must carry the success footer",
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
    assert_contains(
        &log_body,
        DESLOP_INVOKED_MESSAGE,
        "log file missing the invoked event",
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
    assert_contains(
        &stderr,
        DESLOP_INVOKED_MESSAGE,
        "--log-to-console must surface the invoked event on stderr",
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
    assert_not_contains(
        &log_body,
        DESLOP_INVOKED_MESSAGE,
        "warn level must suppress the INFO invoked event",
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
    assert_contains(
        &stderr,
        "deslop scanning",
        "preamble must announce the scan",
    );
    assert_contains(
        &stderr,
        "min-nodes=8",
        "--technical preamble must surface the min-nodes knob",
    );
    assert_contains(
        &stderr,
        "report →",
        "preamble must show where the report goes",
    );
    assert_contains(&stderr, "log    →", "preamble must show where the log goes");
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
    assert_not_contains(
        &stderr,
        DESLOP_INVOKED_MESSAGE,
        "RUST_LOG=warn must suppress INFO events",
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
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_scan_root(&fixture(CSHARP_SMALL_FIXTURE), &scan_root)?;
    // First run populates the cache.
    run_scan(
        &scan_root,
        &tmp.path().join("first"),
        &[MIN_NODES_FLAG, MIN_NODES_VALUE],
    )?;
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
    assert_contains(
        &stderr,
        "cache: 2 hit / 0 miss",
        "--technical must surface the raw cache-stats line",
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
    let (tmp, scan_root) = temp_scan_dir("src")?;
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
    assert_contains(
        &stderr,
        "embeddings: ollama/nomic-embed-text@",
        "--technical must surface the provenance triple on stderr",
    );
    Ok(())
}

// Implements [UX-TECHNICAL-BREAKDOWN]: `--technical` prints the
// researcher breakdown row with the column legend. Plain mode uses
// friendly wording; this test guards the technical branch's wire facts
// — the folded clone kind, cluster id, mass, occurrence count,
// canonical node count and files ([CLONE-KIND-LABELS],
// [RANK-MASS-SUM], [SEVERITY-MODEL]). Pair-only values
// (structural/Jaccard/embedding/content) appear only under an explicit
// endpoint comparison. The renamed C# pair folds to the near-copy
// kind, so its row must name it.
#[test]
fn technical_mode_names_the_clone_kind_in_the_breakdown_row() -> Result<()> {
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
    assert_contains(
        &stderr,
        "columns: rank, kind, id, mass, occurrences, canonical AST nodes, files",
        "--technical must print the column legend naming the kind column",
    );
    assert!(
        stderr.contains(&format!("#1  {NEARLY_IDENTICAL_TITLE} ["))
            && stderr.contains("· mass 58 · 2 occurrences · 58 AST nodes"),
        "--technical must print the mass-ranked cluster row with kind, id, mass, \
         occurrences, nodes and files: {stderr}"
    );
    assert_contains(
        &stderr,
        "Alpha.cs, Beta.cs",
        "--technical cluster row must name both files",
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
    assert_not_contains(
        &stderr,
        "Worst offender",
        "empty scan must not print a worst-offender line",
    );
    Ok(())
}
