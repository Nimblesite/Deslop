use crate::support::*;

#[test]
fn default_run_writes_log_to_timestamped_file_not_stderr() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .env_remove("RUST_LOG")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
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
        log_body.contains("deslop invoked"),
        "log file missing the invoked event: {log_body}"
    );
    Ok(())
}

// Implements [UX-LOG-CONSOLE]: `--log-to-console` routes log events
// back to stderr instead of the file.
#[test]
fn log_to_console_flag_routes_events_to_stderr() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .env_remove("RUST_LOG")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--log-to-console")
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        stderr.contains("deslop invoked"),
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .env_remove("RUST_LOG")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--log-level")
        .arg("warn")
        .arg("--no-color")
        .assert()
        .success();
    let log_path = find_timestamped_logs(tmp.path())?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no timestamped log file written"))?;
    let log_body = fs::read_to_string(&log_path)?;
    assert!(
        !log_body.contains("deslop invoked"),
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--technical")
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .env("DESLOP_FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .env("RUST_LOG", "warn")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--log-to-console")
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        !stderr.contains("deslop invoked"),
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .env("NO_COLOR", "1")
        .env("DESLOP_FORCE_COLOR", "1")
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
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
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    // First run populates the cache.
    let mut first = Command::cargo_bin("deslop")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .assert()
        .success();
    let mut second = Command::cargo_bin("deslop")?;
    let assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--incremental")
        .arg("--technical")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        stderr.contains("cache: 2 hit / 0 miss"),
        "--technical must surface the raw cache-stats line: {stderr}"
    );
    Ok(())
}

// Implements [UX-TECHNICAL-EMBEDDINGS]: `--technical` with a live
// embedding provider prints the provenance triple
// `provider/model@version (N-d)` on stderr. The stub provider is
// deterministic so the test doesn't depend on Ollama being
// installed.
#[test]
fn technical_mode_surfaces_embedding_provenance_line() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("stub")
        .arg("--technical")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        stderr.contains("embeddings: stub/blake3-stub@v1"),
        "--technical must surface the provenance triple on stderr: {stderr}"
    );
    Ok(())
}

// Implements [UX-TECHNICAL-BREAKDOWN]: `--technical` prints the
// researcher breakdown row with Type-1/2/3 labels. Plain mode uses
// friendly wording; this test guards the taxonomy string the
// technical branch emits.
#[test]
fn technical_mode_uses_type_taxonomy_in_breakdown_row() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--technical")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        stderr.contains("1 × Nearly identical code [Type-3]"),
        "--technical must print the Type-taxonomy breakdown: {stderr}"
    );
    assert!(
        stderr.contains("#1  ● Nearly identical code [Type-3]"),
        "--technical must print Type taxonomy in the ranked row: {stderr}"
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let assertion = cmd
        .arg(&empty)
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--no-color")
        .assert()
        .success();
    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?.to_owned();
    assert!(
        !stderr.contains("Worst offender"),
