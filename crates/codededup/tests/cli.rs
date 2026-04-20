//! End-to-end CLI tests. Per `CLAUDE.md`, these are the only kind of
//! test the project ships — driving the binary as a black box against
//! fixture input and asserting on rendered outputs (JSON / text / HTML
//! on disk) and exit codes.
//!
//! After P4.1, the CLI writes the three formats to files under an
//! `--output <prefix>` path (or `codededup-report.{json,txt,html}` in
//! CWD by default). These tests pass an explicit `--output` pointed at
//! a `tempfile::tempdir` so nothing leaks into the repository.
//!
//! After P4.2, exclusion semantics are verified: `exclude` drops files
//! from discovery entirely, `report_hide` keeps clusters visible when a
//! non-hidden file duplicates hidden code but drops them when every
//! member is hidden ([EXCLUSION-CONFIG]).

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;

/// Returns the absolute path of a fixture under `tests/fixtures/<name>`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Runs the binary in `<tmp>` with `--output <tmp>/report`, returning
/// the three on-disk paths the CLI should have written.
struct RunOutputs {
    /// Path to `<tmp>/report.json`.
    json: PathBuf,
    /// Path to `<tmp>/report.txt`.
    txt: PathBuf,
    /// Path to `<tmp>/report.html`.
    html: PathBuf,
}

/// Renders the three output paths for an `--output <dir>/report` layout.
fn outputs_under(dir: &Path) -> RunOutputs {
    let base = dir.join("report");
    RunOutputs {
        json: with_ext(&base, "json"),
        txt: with_ext(&base, "txt"),
        html: with_ext(&base, "html"),
    }
}

/// Appends `.<ext>` to `base` by cloning and replacing the file name.
fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".");
    name.push(ext);
    path.set_file_name(name);
    path
}

// Implements [CLI-INVOCATION-VERSION]: `codededup --version` prints the
// binary name and exits 0.
#[test]
fn prints_version_and_exits_zero() -> Result<()> {
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("codededup"));
    Ok(())
}

// Implements [CLI-INVOCATION-HELP]: `--help` advertises the configurable
// flags so agents can discover the tuning surface.
#[test]
fn prints_help_and_mentions_min_nodes_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--min-nodes"))
        .stdout(contains("--nojson"))
        .stdout(contains("--notext"))
        .stdout(contains("--nohtml"))
        .stdout(contains("--from-report"))
        .stdout(contains("--config"))
        .stdout(contains("--embeddings"))
        .stdout(contains("--embedding-provider"))
        .stdout(contains("--embedding-model"))
        .stdout(contains("--embedding-endpoint"));
    Ok(())
}

// Implements [CLI-INVOCATION-PATH]: passing an empty directory must not
// panic and must exit 0.
#[test]
fn accepts_path_argument_without_panicking() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(tmp.path())
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    assert!(out.json.exists(), "json missing at {}", out.json.display());
    assert!(out.txt.exists(), "txt missing at {}", out.txt.display());
    assert!(out.html.exists(), "html missing at {}", out.html.display());
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED]: the default run emits JSON, text,
// and HTML side by side. All three must carry the v2 schema fields.
#[test]
fn default_run_emits_all_three_formats() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"report_schema_version\": 2"),
        "schema version missing: {json}"
    );
    assert!(json.contains("\"schema_doc\""), "schema_doc missing");
    assert!(json.contains("\"action_hints\""), "action_hints missing");
    assert!(
        json.contains("\"interpretation\""),
        "interpretation missing"
    );
    assert!(json.contains("\"hidden\""), "hidden flag missing");
    let txt = fs::read_to_string(&out.txt)?;
    assert!(txt.contains("codededup"), "text header missing: {txt}");
    let html = fs::read_to_string(&out.html)?;
    assert!(html.contains("<!doctype html>"), "html doctype missing");
    assert!(html.contains("Action hints"), "html action hints missing");
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED] suppression flags: `--nojson
// --nohtml` leaves only the text output behind.
#[test]
fn suppression_flags_leave_only_enabled_formats() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--nojson")
        .arg("--nohtml")
        .assert()
        .success();
    assert!(!out.json.exists(), "json should be suppressed");
    assert!(!out.html.exists(), "html should be suppressed");
    assert!(out.txt.exists(), "txt should still exist");
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED]: suppressing every format is an
// error — silent runs are never useful.
#[test]
fn suppressing_every_format_is_an_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--nojson")
        .arg("--notext")
        .arg("--nohtml")
        .assert()
        .failure()
        .stderr(contains("must remain enabled"));
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED] `--from-report`: analysis is
// skipped and the derived formats are re-rendered from the canonical
// JSON. Exercises the deserialize path on the Report struct.
#[test]
fn from_report_rerenders_without_analysing() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut first = Command::cargo_bin("codededup")?;
    let _assertion = first
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--notext")
        .arg("--nohtml")
        .assert()
        .success();
    assert!(out.json.exists());
    let rendered_dir = tempfile::tempdir()?;
    let rerender = outputs_under(rendered_dir.path());
    let mut second = Command::cargo_bin("codededup")?;
    let _assertion = second
        .arg(tmp.path())
        .arg("--from-report")
        .arg(&out.json)
        .arg("--output")
        .arg(rendered_dir.path().join("report"))
        .arg("--nojson")
        .assert()
        .success();
    assert!(rerender.txt.exists(), "txt not re-rendered");
    assert!(rerender.html.exists(), "html not re-rendered");
    Ok(())
}

// Implements [PIPELINE-CLUSTER-EXACT] + [PIPELINE-NORMALIZE-AST]: two
// C# files with the same structure but renamed identifiers (Type-2
// clone) must produce a cluster of size 2 in the canonical JSON.
#[test]
fn detects_type2_clone_in_csharp_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("Alpha.cs"));
    assert!(json.contains("Beta.cs"));
    assert!(json.contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Rust: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_rust_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("rust-small"))
        .arg("--min-nodes")
        .arg("10")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.rs"));
    assert!(json.contains("beta.rs"));
    assert!(json.contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Python: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_python_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("python-small"))
        .arg("--min-nodes")
        .arg("10")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.py"));
    assert!(json.contains("beta.py"));
    assert!(json.contains("\"structural\": 1.0"));
    Ok(())
}

// Implements multi-language dispatch — three files routed by extension
// in one run.
#[test]
fn handles_mixed_language_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("mixed-small"))
        .arg("--min-nodes")
        .arg("10")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 3"));
    assert!(json.contains("Lib.cs"));
    assert!(json.contains("lib.rs"));
    assert!(json.contains("lib.py"));
    Ok(())
}

// Implements [DECISION-TYPE3-TWO-PASS] + [FUSION-STRATEGY-MAX-SUM]:
// Type-3 near-miss cross-file cluster with `structural=0.0`.
#[test]
fn detects_type3_clone_in_csharp_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-type3"))
        .arg("--min-nodes")
        .arg("15")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("Delta.cs"));
    assert!(json.contains("Epsilon.cs"));
    assert!(json.contains("\"structural\": 0.0"));
    assert!(json.contains("\"token_jaccard\""));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `exclude` tier: a file matched by the
// exclude pattern is never parsed, never counted in `files_analysed`.
#[test]
fn exclude_pattern_drops_file_from_discovery() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("codededup.toml");
    fs::write(&config, "[defaults]\nexclude = [\"**/Beta.cs\"]\n")?;
    let mut cmd = Command::cargo_bin("codededup")?;
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
    let config = tmp.path().join("codededup.toml");
    fs::write(&config, "[defaults]\nreport_hide = [\"**/Beta.cs\"]\n")?;
    let mut cmd = Command::cargo_bin("codededup")?;
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
    let config = tmp.path().join("codededup.toml");
    fs::write(
        &config,
        "[language.csharp]\nreport_hide = [\"**/Beta.cs\"]\n",
    )?;
    let mut cmd = Command::cargo_bin("codededup")?;
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
    let config = tmp.path().join("codededup.toml");
    fs::write(
        &config,
        "[language.python]\nexclude = [\"**/*.py\"]\n\n[language.csharp]\nexclude = [\"**/Beta.cs\"]\n",
    )?;
    let mut cmd = Command::cargo_bin("codededup")?;
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
// `<scan_root>/.codededup.toml` automatically.
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
        scan_root.join(".codededup.toml"),
        "[defaults]\nexclude = [\"**/Beta.cs\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
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

// Implements [EXCLUSION-CONFIG] error reporting: a malformed TOML file
// must exit non-zero with the upstream parse error surfaced.
#[test]
fn malformed_config_file_reports_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let config = tmp.path().join("codededup.toml");
    fs::write(&config, "not valid toml = = =\n")?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--config")
        .arg(&config)
        .assert()
        .failure()
        .stderr(contains("failed to parse exclusion config"));
    Ok(())
}

// Implements default output paths: running without `--output` writes
// `codededup-report.{json,txt,html}` into the current working
// directory. We run the command with `current_dir(tempdir)` so the
// artefacts don't leak into the repo.
#[test]
fn default_output_written_to_current_directory() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .current_dir(tmp.path())
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .assert()
        .success();
    assert!(tmp.path().join("codededup-report.json").exists());
    assert!(tmp.path().join("codededup-report.txt").exists());
    assert!(tmp.path().join("codededup-report.html").exists());
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER]: when `--embeddings=off` (the
// default), the report carries `embedding_provenance: null` and the
// text renderer prints `embeddings: off`. Keeps the deterministic CI
// path guaranteed to run without an Ollama daemon.
#[test]
fn default_run_records_embeddings_off_provenance() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"embedding_provenance\": null"),
        "default run must record embeddings=off: {json}"
    );
    let txt = fs::read_to_string(&out.txt)?;
    assert!(txt.contains("embeddings: off"), "text provenance missing");
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=required` fails
// hard when the provider is unreachable. Uses an endpoint we know
// cannot resolve (port 1) so the probe always fails regardless of
// whether Ollama happens to be running on the developer machine.
#[test]
fn embeddings_required_hard_fails_when_provider_unreachable() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-endpoint")
        .arg("http://127.0.0.1:1")
        .assert()
        .failure()
        .stderr(contains("unreachable"));
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=auto` falls back
// silently when the provider is unreachable — the pipeline must
// still produce a report with `embedding_provenance: null`.
#[test]
fn embeddings_auto_falls_back_when_provider_unreachable() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-endpoint")
        .arg("http://127.0.0.1:1")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"embedding_provenance\": null"),
        "auto must fall back to off when provider is down: {json}"
    );
    Ok(())
}

// Implements [CLI-ARG-EMBEDDINGS]: invalid `--embeddings` values are
// rejected with a clear error message.
#[test]
fn embeddings_flag_rejects_unknown_values() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("maybe")
        .assert()
        .failure()
        .stderr(contains("invalid --embeddings value"));
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] stub provider: `--embedding-provider=stub
// --embeddings=required` runs the full embedding pipeline with a
// deterministic in-process provider. Exercises the HNSW pair
// generator, the cache round-trip, and the provenance rendering
// without needing a live Ollama.
#[test]
fn stub_provider_records_provenance_and_runs_embedding_pass() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-type3"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("stub")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"stub\""),
        "provenance provider_id missing: {json}"
    );
    assert!(
        json.contains("\"model_id\": \"blake3-stub\""),
        "provenance model_id missing"
    );
    assert!(
        json.contains("\"model_version\": \"v1\""),
        "provenance model_version missing"
    );
    let txt = fs::read_to_string(&out.txt)?;
    assert!(
        txt.contains("embeddings: stub/blake3-stub@v1"),
        "text provenance missing: {txt}"
    );
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("embeddings: stub/blake3-stub@v1"),
        "html provenance missing"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] cache round-trip: a second run
// against the same scan root must re-use the on-disk embedding
// cache rather than re-embedding from scratch. We verify the cache
// directory exists after the first run and the second run still
// succeeds (the stub provider is deterministic, so a cache miss
// would still produce the same vectors — but we check the directory
// to prove the cache path actually wrote files).
#[test]
fn stub_provider_populates_embedding_cache() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-small"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let mut first = Command::cargo_bin("codededup")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("stub")
        .assert()
        .success();
    let cache_dir = scan_root
        .join(".codededup-cache")
        .join("embeddings")
        .join("stub")
        .join("blake3-stub")
        .join("v1");
    assert!(
        cache_dir.is_dir(),
        "embedding cache directory missing: {}",
        cache_dir.display()
    );
    let cached_files = fs::read_dir(&cache_dir)?.count();
    assert!(
        cached_files > 0,
        "cache dir has no entries: {}",
        cache_dir.display()
    );
    let mut second = Command::cargo_bin("codededup")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("stub")
        .assert()
        .success();
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] `--embeddings=auto` with a
// reachable provider: the pass succeeds and the report carries the
// provenance. Complements the failure-fallback test.
#[test]
fn stub_provider_under_auto_mode_runs_embedding_pass() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-small"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("stub")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"stub\""),
        "auto mode with reachable provider must record provenance: {json}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] unknown-provider rejection.
#[test]
fn unknown_embedding_provider_is_rejected() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("imaginary-provider")
        .assert()
        .failure()
        .stderr(contains("unknown embedding provider"));
    Ok(())
}

// OLLAMA-LIVE TEST — requires a running local Ollama daemon on
// 127.0.0.1:11434 with the `nomic-embed-text` model pulled.
// The `ollama_` name prefix is the marker: `make ci` filters it out
// via `cargo test ... -- --skip ollama_`; `make ci-ollama` runs it
// via `cargo test ollama_`. Implements [FUSION-EMBED-PROVIDER]
// Type-4 detection end-to-end with the real Ollama provider.
#[test]
fn ollama_type4_pass_records_provenance_and_produces_signal() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-type4"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(
        json.contains("\"provider_id\": \"ollama\""),
        "provenance missing provider_id: {json}"
    );
    assert!(
        json.contains("\"model_id\": \"nomic-embed-text\""),
        "provenance missing model_id: {json}"
    );
    assert!(
        json.contains("Recursive.cs"),
        "expected Recursive.cs in report: {json}"
    );
    assert!(
        json.contains("Iterative.cs"),
        "expected Iterative.cs in report: {json}"
    );
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED]: `--output <nested-dir>/report`
// creates missing parent directories rather than erroring out.
#[test]
fn output_path_with_missing_parent_is_created() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let base = tmp.path().join("a").join("b").join("c").join("report");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(&base)
        .assert()
        .success();
    assert!(
        base.with_extension("json").exists(),
        "nested-parent json missing"
    );
    assert!(
        base.with_extension("txt").exists(),
        "nested-parent txt missing"
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `report_hide` drops a cluster whose
// members are all hidden and increments `clusters_hidden`.
#[test]
fn report_hide_drops_cluster_when_all_members_hidden() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("codededup.toml");
    fs::write(&config, "[defaults]\nreport_hide = [\"**/*.cs\"]\n")?;
    let mut cmd = Command::cargo_bin("codededup")?;
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
    assert!(
        !json.contains("\"hidden\": false"),
        "every member should be hidden: {json}"
    );
    assert!(
        !json.contains("\"structural\": 1.0"),
        "hidden-only cluster must be dropped from visible list"
    );
    assert!(
        json.contains("\"clusters_hidden\": 1"),
        "clusters_hidden must count the suppressed cluster"
    );
    Ok(())
}
