use crate::support::*;

/// Returns the command scanning the `csharp-small` fixture with
/// `--config <tmp>/<file_name>`, writing `body` there first when `Some`
/// — an absent file is what the missing-config path needs. Reports land
/// at `<tmp>/report.*`; the caller adds its flags and asserts.
fn config_command(tmp: &Path, file_name: &str, body: Option<&str>) -> Result<Command> {
    let config = tmp.join(file_name);
    if let Some(body) = body {
        fs::write(&config, body)?;
    }
    let mut cmd = fixture_command("csharp-small", &tmp.join("report"))?;
    let _cmd = cmd.arg("--config").arg(&config);
    Ok(cmd)
}

/// Runs [`config_command`] with `config_body` at `--min-nodes 8`,
/// asserts success, and returns the JSON report body — the
/// `--config`-driven scenarios differ only in the TOML they supply.
fn run_with_config(config_body: &str) -> Result<String> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = config_command(tmp.path(), "deslop.toml", Some(config_body))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    Ok(fs::read_to_string(outputs_under(tmp.path()).json)?)
}

/// The scan root [`seeded_scan_command`] materialises under `tmp`.
fn scan_root_under(tmp: &Path) -> PathBuf {
    tmp.join("scan-root")
}

/// Copies `csharp-small` fixture files into `scan_root`: each
/// `(directory, file name)` pair lands at
/// `<scan_root>/<directory>/<file name>`, an empty directory meaning
/// the scan root itself.
fn place_fixture_files(scan_root: &Path, placements: &[(&str, &str)]) -> Result<()> {
    placements.iter().try_for_each(|&(directory, file_name)| {
        let target = scan_root.join(directory);
        fs::create_dir_all(&target)?;
        let source = fixture("csharp-small").join(file_name);
        let _bytes = fs::copy(source, target.join(file_name))?;
        Ok(())
    })
}

/// Seeds [`scan_root_under`] with `placements` and, when `config_body`
/// is `Some`, a `.deslop.toml` holding it — the default config filename
/// the pipeline discovers without `--config`. Returns the command
/// scanning that root at `--min-nodes 8`, reporting to `<tmp>/report.*`.
fn seeded_scan_command(
    tmp: &Path,
    placements: &[(&str, &str)],
    config_body: Option<&str>,
) -> Result<Command> {
    let scan_root = scan_root_under(tmp);
    fs::create_dir_all(&scan_root)?;
    place_fixture_files(&scan_root, placements)?;
    if let Some(body) = config_body {
        fs::write(scan_root.join(".deslop.toml"), body)?;
    }
    let mut cmd = deslop_command(&scan_root, &tmp.join("report"))?;
    let _cmd = cmd.args(["--min-nodes", "8"]);
    Ok(cmd)
}

/// The `hidden` flag of the last occurrence, across every cluster,
/// whose path ends with `suffix`; `None` when no occurrence matches.
/// A later match overwrites an earlier one, so the fold keeps the last.
fn hidden_flag_for(report: &Value, suffix: &str) -> Option<bool> {
    field(report, "clusters")
        .as_array()?
        .iter()
        .flat_map(|cluster| {
            field(cluster, "occurrences")
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(suffix))
        })
        .fold(None, |_earlier, occurrence| {
            Some(field(occurrence, "hidden").as_bool().unwrap_or(false))
        })
}

#[test]
fn exclude_pattern_drops_file_from_discovery() -> Result<()> {
    let json = run_with_config("[defaults]\nexclude = [\"**/Beta.cs\"]\n")?;
    assert!(
        json.contains("\"files_analysed\": 1"),
        "exclude should drop Beta.cs, leaving one file: {json}"
    );
    assert!(
        !json.contains("Beta.cs"),
        "Beta.cs must not appear when excluded"
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `report_hide` keeps the cluster visible
// when a non-hidden member duplicates hidden code — the "regular code
// duplicates generated code" scenario. The cluster survives, with the
// hidden member flagged.
#[test]
fn report_hide_keeps_mixed_cluster_and_flags_hidden_occurrence() -> Result<()> {
    let json = run_with_config("[defaults]\nreport_hide = [\"**/Beta.cs\"]\n")?;
    assert!(
        json.contains("\"files_analysed\": 2"),
        "report_hide must still analyse the file"
    );
    assert!(json.contains("Alpha.cs"));
    assert!(json.contains("Beta.cs"));
    assert!(
        json.contains("\"hidden\": true"),
        "hidden occurrence must be flagged"
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] per-language overlay: a
// `[language.csharp]` section adds to `[defaults]` without replacing
// it. Here we only set a per-language `report_hide` so the default
// section stays empty — proves the overlay matcher path.
#[test]
fn report_hide_per_language_overlay_flags_csharp_only() -> Result<()> {
    let json = run_with_config("[language.csharp]\nreport_hide = [\"**/Beta.cs\"]\n")?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("\"hidden\": true"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] per-language `exclude` overlay: the
// Python rules should not affect a C# file.
#[test]
fn exclude_per_language_overlay_scoped_to_its_language() -> Result<()> {
    let json = run_with_config(
        "[language.python]\nexclude = [\"**/*.py\"]\n\n[language.csharp]\nexclude = [\"**/Beta.cs\"]\n",
    )?;
    assert!(json.contains("\"files_analysed\": 1"));
    assert!(!json.contains("Beta.cs"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] default filename discovery: when no
// `--config` is passed, the pipeline picks up
// `<scan_root>/.deslop.toml` automatically.
#[test]
fn default_config_file_in_scan_root_is_loaded() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = seeded_scan_command(
        tmp.path(),
        &[("", "Alpha.cs"), ("", "Beta.cs")],
        Some("[defaults]\nexclude = [\"**/Beta.cs\"]\n"),
    )?;
    let _assertion = cmd.assert().success();
    let json = fs::read_to_string(outputs_under(tmp.path()).json)?;
    assert!(json.contains("\"files_analysed\": 1"));
    Ok(())
}

// Implements [PIPELINE-DISCOVER-FILES]: files without an extension
// (e.g. `Makefile`) are skipped silently — the discovery walker
// has no language plug-in to hand them to. Covers the
// `lowercase_extension -> None` branch in the discovery loop.
#[test]
fn files_without_extensions_are_skipped_silently() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = seeded_scan_command(tmp.path(), &[("", "Alpha.cs")], None)?;
    let scan_root = scan_root_under(tmp.path());
    fs::write(scan_root.join("Makefile"), "all:\n\techo hi\n")?;
    fs::write(scan_root.join("README"), "nothing to see here\n")?;
    let _assertion = cmd.assert().success();
    let json = fs::read_to_string(outputs_under(tmp.path()).json)?;
    assert!(
        json.contains("\"files_analysed\": 1"),
        "Makefile / README must be filtered before the language dispatch: {json}"
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] missing-config error path: pointing
// `--config` at a path that doesn't exist must surface the IO error
// via the CLI error footer, not silently fall back to an empty
// config.
#[test]
fn missing_config_file_reports_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = config_command(tmp.path(), "does-not-exist.toml", None)?;
    let _assertion = cmd
        .arg("--no-color")
        .assert()
        .failure()
        .stderr(contains("failed"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] invalid-pattern error path: an
// ill-formed gitignore pattern (here `[unclosed`) must fail the
// config compile step, not crash.
#[test]
fn invalid_exclude_pattern_reports_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let body = "[defaults]\nexclude = [\"[unclosed\"]\n";
    let mut cmd = config_command(tmp.path(), "deslop.toml", Some(body))?;
    let _assertion = cmd
        .arg("--no-color")
        .assert()
        .failure()
        .stderr(contains("failed"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] error reporting: a malformed TOML file
// must exit non-zero with the upstream parse error surfaced.
#[test]
fn malformed_config_file_reports_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = config_command(tmp.path(), "deslop.toml", Some("not valid toml = = =\n"))?;
    let _assertion = cmd
        .arg("--no-color")
        .assert()
        .failure()
        .stderr(contains("failed to parse exclusion config"))
        .stderr(contains("failed"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] (#138): `.deslop.toml` patterns are
// scan-root-relative, not absolute. A pattern `subdir/**` (no `**/`
// prefix) must hide files at `<scan_root>/subdir/...`. The bundled
// CLI in Basilisk failed because `GitignoreBuilder::new("/")` rooted
// every matcher at `/` and matched against absolute paths.
#[test]
fn report_hide_pattern_is_scan_root_relative() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = seeded_scan_command(
        tmp.path(),
        &[("benchmarks/fixtures", "Alpha.cs"), ("src", "Beta.cs")],
        Some("[defaults]\nreport_hide = [\"benchmarks/fixtures/**\"]\n"),
    )?;
    let _assertion = cmd.assert().success();
    let report = read_json_report(&outputs_under(tmp.path()).json)?;
    assert_eq!(
        hidden_flag_for(&report, "Alpha.cs"),
        Some(true),
        "scan-root-relative pattern `benchmarks/fixtures/**` must hide Alpha.cs at <scan_root>/benchmarks/fixtures/Alpha.cs",
    );
    assert_eq!(
        hidden_flag_for(&report, "Beta.cs"),
        Some(false),
        "Beta.cs at <scan_root>/src/Beta.cs must remain visible",
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] (#138): the same scan-root-relative
// rule applies to `exclude`. A pattern `subdir/**` must drop files
// at `<scan_root>/subdir/...` from discovery without requiring a
// `**/` prefix.
#[test]
fn exclude_pattern_is_scan_root_relative() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = seeded_scan_command(
        tmp.path(),
        &[("benchmarks/fixtures", "Alpha.cs"), ("src", "Beta.cs")],
        Some("[defaults]\nexclude = [\"benchmarks/fixtures/**\"]\n"),
    )?;
    let _assertion = cmd.assert().success();
    let body = fs::read_to_string(outputs_under(tmp.path()).json)?;
    assert!(
        body.contains("\"files_analysed\": 1"),
        "scan-root-relative `exclude` pattern must drop benchmarks/fixtures/Alpha.cs and leave only Beta.cs analysed: {body}",
    );
    assert!(
        !body.contains("benchmarks/fixtures/Alpha.cs"),
        "Alpha.cs under benchmarks/fixtures must not appear in the report: {body}",
    );
    Ok(())
}

// Implements [OUTPUT-DIR]: running without `--output` writes
// `.deslop/deslop-report.{json,txt,html}` under the *scan root* — not
// the working directory — so the CLI, LSP, and MCP all address one
// location per workspace. Logs go to `.deslop/logs/`, and the cache to
// `.deslop/cache/`, so `.deslop/` is the single directory a user has to
// gitignore. The command runs from a separate working directory to
// prove the outputs follow the scan root rather than the CWD.
#[test]
fn default_output_written_to_deslop_dir_under_scan_root() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let cwd = tmp.path().join("elsewhere");
    fs::create_dir_all(&cwd)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .current_dir(&cwd)
        .arg(&scan_root)
        .args(["--min-nodes", "8"])
        .assert()
        .success();
    let output_dir = scan_root.join(".deslop");
    assert!(output_dir.join("deslop-report.json").exists());
    assert!(output_dir.join("deslop-report.txt").exists());
    assert!(output_dir.join("deslop-report.html").exists());
    assert_eq!(
        find_timestamped_logs(&output_dir)?.len(),
        1,
        "the run's log belongs in .deslop/logs/, not loose in .deslop/",
    );
    assert!(
        output_dir.join("cache").join("fingerprints").is_dir(),
        "--incremental must cache under .deslop/cache/, not a sibling .deslop-cache/",
    );
    assert!(
        !scan_root.join(".deslop-cache").exists(),
        "the pre-[OUTPUT-DIR] cache directory must no longer be written",
    );
    for stray in [
        "deslop-report.json",
        "deslop-report.txt",
        "deslop-report.html",
    ] {
        assert!(
            !cwd.join(stray).exists(),
            "{stray} must not be dropped in the working directory",
        );
    }
    Ok(())
}
