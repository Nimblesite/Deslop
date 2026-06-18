use crate::support::*;

/// Writes `config_body` to `<tmp>/deslop.toml`, runs the CLI over the
/// `csharp-small` fixture with `--min-nodes 8 --config <that file>`,
/// asserts success, and returns the JSON report body as a string. Used
/// by the `--config`-driven exclusion/hide scenarios that differ only
/// in the TOML they supply.
fn run_with_config(config_body: &str) -> Result<String> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, config_body)?;
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--config"])
        .arg(&config)
        .assert()
        .success();
    Ok(fs::read_to_string(&out.json)?)
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
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let _alpha_bytes = fs::copy(
        fixture("csharp-small").join("Alpha.cs"),
        scan_root.join("Alpha.cs"),
    )?;
    let _beta_bytes = fs::copy(
        fixture("csharp-small").join("Beta.cs"),
        scan_root.join("Beta.cs"),
    )?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[defaults]\nexclude = [\"**/Beta.cs\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let json = fs::read_to_string(&out.json)?;
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
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let _alpha_bytes = fs::copy(
        fixture("csharp-small").join("Alpha.cs"),
        scan_root.join("Alpha.cs"),
    )?;
    fs::write(scan_root.join("Makefile"), "all:\n\techo hi\n")?;
    fs::write(scan_root.join("README"), "nothing to see here\n")?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
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
    let missing = tmp.path().join("does-not-exist.toml");
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .arg("--config")
        .arg(&missing)
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
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults]\nexclude = [\"[unclosed\"]\n")?;
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .arg("--config")
        .arg(&config)
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
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "not valid toml = = =\n")?;
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .arg("--config")
        .arg(&config)
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
    let scan_root = tmp.path().join("repo");
    let hidden_dir = scan_root.join("benchmarks").join("fixtures");
    let visible_dir = scan_root.join("src");
    fs::create_dir_all(&hidden_dir)?;
    fs::create_dir_all(&visible_dir)?;
    let _alpha_bytes = fs::copy(
        fixture("csharp-small").join("Alpha.cs"),
        hidden_dir.join("Alpha.cs"),
    )?;
    let _beta_bytes = fs::copy(
        fixture("csharp-small").join("Beta.cs"),
        visible_dir.join("Beta.cs"),
    )?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[defaults]\nreport_hide = [\"benchmarks/fixtures/**\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let report = read_json_report(&out.json)?;
    let clusters = field(&report, "clusters")
        .as_array()
        .context("clusters array")?;
    let mut alpha_hidden: Option<bool> = None;
    let mut beta_hidden: Option<bool> = None;
    for cluster in clusters {
        let Some(occurrences) = field(cluster, "occurrences").as_array() else {
            continue;
        };
        for occurrence in occurrences {
            let path = field(occurrence, "path").as_str().unwrap_or("");
            let hidden = field(occurrence, "hidden").as_bool().unwrap_or(false);
            if path.ends_with("Alpha.cs") {
                alpha_hidden = Some(hidden);
            } else if path.ends_with("Beta.cs") {
                beta_hidden = Some(hidden);
            }
        }
    }
    assert_eq!(
        alpha_hidden,
        Some(true),
        "scan-root-relative pattern `benchmarks/fixtures/**` must hide Alpha.cs at <scan_root>/benchmarks/fixtures/Alpha.cs",
    );
    assert_eq!(
        beta_hidden,
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
    let scan_root = tmp.path().join("repo");
    let excluded_dir = scan_root.join("benchmarks").join("fixtures");
    let kept_dir = scan_root.join("src");
    fs::create_dir_all(&excluded_dir)?;
    fs::create_dir_all(&kept_dir)?;
    let _alpha_bytes = fs::copy(
        fixture("csharp-small").join("Alpha.cs"),
        excluded_dir.join("Alpha.cs"),
    )?;
    let _beta_bytes = fs::copy(
        fixture("csharp-small").join("Beta.cs"),
        kept_dir.join("Beta.cs"),
    )?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[defaults]\nexclude = [\"benchmarks/fixtures/**\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let body = fs::read_to_string(&out.json)?;
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

// Implements default output paths: running without `--output` writes
// `deslop-report.{json,txt,html}` into the current working
// directory. We run the command with `current_dir(tempdir)` so the
// artefacts don't leak into the repo.
#[test]
fn default_output_written_to_current_directory() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .current_dir(tmp.path())
        .arg(fixture("csharp-small"))
        .args(["--min-nodes", "8"])
        .assert()
        .success();
    assert!(tmp.path().join("deslop-report.json").exists());
    assert!(tmp.path().join("deslop-report.txt").exists());
    assert!(tmp.path().join("deslop-report.html").exists());
    Ok(())
}
