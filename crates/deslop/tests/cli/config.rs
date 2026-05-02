use crate::support::*;

#[test]
fn exclude_pattern_drops_file_from_discovery() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults]\nexclude = [\"**/Beta.cs\"]\n")?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
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
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults]\nreport_hide = [\"**/Beta.cs\"]\n")?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
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
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(
        &config,
        "[language.csharp]\nreport_hide = [\"**/Beta.cs\"]\n",
    )?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("\"hidden\": true"));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] per-language `exclude` overlay: the
// Python rules should not affect a C# file.
#[test]
fn exclude_per_language_overlay_scoped_to_its_language() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(
        &config,
        "[language.python]\nexclude = [\"**/*.py\"]\n\n[language.csharp]\nexclude = [\"**/Beta.cs\"]\n",
    )?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .arg("--no-color")
        .assert()
        .failure()
        .stderr(contains("failed to parse exclusion config"))
        .stderr(contains("failed"));
    Ok(())
}

// Implements default output paths: running without `--output` writes
// `deslop-report.{json,txt,html}` into the current working
// directory. We run the command with `current_dir(tempdir)` so the
// artefacts don't leak into the repo.
#[test]
