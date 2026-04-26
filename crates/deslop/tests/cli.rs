//! End-to-end CLI tests. Per `CLAUDE.md`, these are the only kind of
//! test the project ships — driving the binary as a black box against
//! fixture input and asserting on rendered outputs (JSON / text / HTML
//! on disk) and exit codes.
//!
//! After P4.1, the CLI writes the three formats to files under an
//! `--output <prefix>` path (or `deslop-report.{json,txt,html}` in
//! CWD by default). These tests pass an explicit `--output` pointed at
//! a `tempfile::tempdir` so nothing leaks into the repository.
//!
//! After P4.2, exclusion semantics are verified: `exclude` drops files
//! from discovery entirely, `report_hide` keeps clusters visible when a
//! non-hidden file duplicates hidden code but drops them when every
//! member is hidden ([EXCLUSION-CONFIG]).

use std::{fmt::Write as _, fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;

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

/// Copies every top-level entry in `src` into a freshly created `dst`.
/// Used by tests that need a mutable scan root seeded from an
/// immutable fixture (cache/embedding tests write siblings next to the
/// sources).
fn seed_scan_root(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Collects every `deslop-*.log` file sitting in `dir`. The default
/// logging path writes a timestamped file next to the report; tests
/// need to locate it without hardcoding the stamp.
fn find_timestamped_logs(dir: &Path) -> Result<Vec<PathBuf>> {
    let matches = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("deslop-")
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
                })
        })
        .collect();
    Ok(matches)
}

// Implements [CLI-INVOCATION-VERSION]: `deslop --version` prints the
// binary name and exits 0.
#[test]
fn prints_version_and_exits_zero() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--version")
        .assert()
        .success()
        .stdout("deslop 0.1.0\n")
        .stderr("");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let output = Command::cargo_bin("deslop")?
        .arg("--version")
        .arg("--json")
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_version_manifest(&value, "deslop", "cli");
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    assert_eq!(value.get("manifestVersion"), Some(&Value::from(1)));
    assert_eq!(value.get("name"), Some(&Value::from(name)));
    assert_eq!(value.get("version"), Some(&Value::from("0.1.0")));
    assert_eq!(value.get("kind"), Some(&Value::from(kind)));
    assert_eq!(value.get("language"), Some(&Value::from("rust")));
    assert_eq!(value.get("product"), Some(&Value::from("deslop")));
}

// Implements [CLI-INVOCATION-HELP]: `--help` advertises the configurable
// flags so agents can discover the tuning surface.
#[test]
fn prints_help_and_mentions_min_nodes_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
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
        .stdout(contains("--embedding-endpoint"))
        .stdout(contains("--log-to-console"))
        .stdout(contains("--log-level"))
        .stdout(contains("--no-color"))
        .stdout(contains("--technical"));
    Ok(())
}

// Implements [CLI-INVOCATION-PATH]: passing an empty directory must not
// panic and must exit 0.
#[test]
fn accepts_path_argument_without_panicking() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
        json.contains("\"report_schema_version\": 1"),
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
    assert!(txt.contains("deslop"), "text header missing: {txt}");
    let html = fs::read_to_string(&out.html)?;
    assert!(html.contains("<!doctype html>"), "html doctype missing");
    assert!(html.contains("Action hints"), "html action hints missing");
    assert!(html.contains("Deslop report"), "html human intro missing");
    assert!(
        html.contains("Duplicate groups"),
        "html cluster section heading missing"
    );
    assert!(
        html.contains("class=\"cluster-card"),
        "html cluster card missing"
    );
    assert!(
        html.contains("class=\"snippet\""),
        "html snippet body missing"
    );
    assert!(
        html.contains("class=\"ln\""),
        "html line-number gutter missing"
    );
    assert!(
        html.contains("--surface-container-low"),
        "html design-system tokens missing"
    );
    Ok(())
}

// Implements [OUTPUT-HUMAN-HTML] preview cap: snippets longer than the
// soft cap render the first window inline and fold the remainder into a
// `<details>` summary so a 300-line clone does not stretch the page.
#[test]
fn long_clone_html_caps_inline_preview_and_folds_rest() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let body = build_long_clone_body(60);
    fs::write(
        scan_root.join("Alpha.cs"),
        wrap_clone_in_class("Alpha", &body),
    )?;
    fs::write(
        scan_root.join("Beta.cs"),
        wrap_clone_in_class("Beta", &body),
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
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("class=\"snippet\""),
        "expected at least one inline snippet block"
    );
    assert!(
        html.contains("more line(s)"),
        "expected the long-clone preview cap to fold extra lines into a details summary, got: {head}",
        head = html.chars().take(400).collect::<String>(),
    );
    Ok(())
}

/// Builds a 60-statement C# method body that is structurally large
/// enough to blow past the snippet preview cap once duplicated.
fn build_long_clone_body(statements: usize) -> String {
    let mut body = String::new();
    for index in 0..statements {
        let _ = writeln!(body, "        var v{index} = {index} + {index};");
    }
    body
}

/// Wraps `body` in a minimal C# class so the C# parser produces a
/// real method-level subtree the clusterer can fingerprint.
fn wrap_clone_in_class(class: &str, body: &str) -> String {
    format!("public class {class} {{\n    public void Run() {{\n{body}    }}\n}}\n")
}

// Implements [OUTPUT-FORMAT-DERIVED] suppression flags: `--nojson
// --nohtml` leaves only the text output behind.
#[test]
fn suppression_flags_leave_only_enabled_formats() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut first = Command::cargo_bin("deslop")?;
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
    let mut second = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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

// Audience: HUMAN. Issue #34. When a human opens two Python test
// files whose functions are structurally unrelated — one synchronous
// test calling `registry.has(...)` in a for-loop assertion block,
// one async helper doing `db.add(UsageEvent(...)); await db.flush()`
// — Deslop must not report them as members of the same clone cluster.
// A human reading the cluster panel should never see two
// dissimilar-shape functions sitting side by side claiming to be
// duplicates; that makes the whole tool untrustworthy.
//
// Positive bound: every cluster in the report has occurrences from a
// single file. Intra-file similarity (e.g. three sibling tests that
// all do `x = registry.get("..."); result = x(...); assert ...`) is
// legitimate and allowed.
#[test]
fn dissimilar_python_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("python-dissimilar-functions"))
        .arg("--min-nodes")
        .arg("10")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    for (index, cluster) in clusters.iter().enumerate() {
        let cluster_id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for occurrence in &occurrences {
            let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let basename = Path::new(path).file_name().map_or_else(
                || path.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            let _inserted = files.insert(basename);
        }
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} ({cluster_id}) spans multiple files {files:?}; \
             the human reader would be confused because the bodies are not similar",
        );
    }
    Ok(())
}

// Returns the byte slice of `path` spanned by `occurrence`'s reported
// `[start_byte, end_byte)`. Used by the prologue-cluster regressions
// to read the source text the report claims is a clone.
fn occurrence_source(scan_root: &Path, occurrence: &serde_json::Value) -> Option<Vec<u8>> {
    let path = occurrence.get("path").and_then(serde_json::Value::as_str)?;
    let start = usize::try_from(
        occurrence
            .get("start_byte")
            .and_then(serde_json::Value::as_u64)?,
    )
    .ok()?;
    let end = usize::try_from(
        occurrence
            .get("end_byte")
            .and_then(serde_json::Value::as_u64)?,
    )
    .ok()?;
    let bytes = fs::read(scan_root.join(path)).ok()?;
    bytes.get(start..end).map(<[u8]>::to_vec)
}

// Returns true when `text` opens with a top-level import/prologue
// construct in any of the languages Deslop currently parses: a Python
// triple-quoted module docstring, `import` / `from` / `if TYPE_CHECKING:`,
// a C# `using` directive or `namespace` declaration, or a Rust
// `use` / `extern crate` statement. The check looks only at the first
// non-whitespace line so a window that starts with prologue and
// extends into real code still counts as prologue-anchored.
fn opens_with_prologue_keyword(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        return true;
    }
    let first_line = trimmed.lines().next().unwrap_or("").trim_start();
    if first_line.starts_with("if TYPE_CHECKING") {
        return true;
    }
    let token = first_line.split_whitespace().next().unwrap_or("");
    matches!(
        token,
        "use" | "using" | "namespace" | "import" | "from" | "extern"
    )
}

// Asserts no cluster in `report` is a cross-file prologue false
// positive: a multi-file cluster whose every occurrence starts on an
// import/use/namespace/docstring line. Drives all three issue-#34
// regression tests (Python, C#, Rust).
fn assert_no_cross_file_prologue_cluster(
    report: &serde_json::Value,
    scan_root: &Path,
    label: &str,
) {
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for cluster in &clusters {
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let cluster_id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut all_prologue = !occurrences.is_empty();
        for occurrence in &occurrences {
            if let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) {
                let _inserted = files.insert(path.to_owned());
            }
            let bytes = occurrence_source(scan_root, occurrence).unwrap_or_default();
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            if !opens_with_prologue_keyword(text) {
                all_prologue = false;
            }
        }
        assert!(
            !(all_prologue && files.len() > 1),
            "{label}: cluster {cluster_id} is a cross-file prologue cluster spanning \
             {files:?}; import / use / namespace / docstring scaffolding must never \
             anchor a cross-file clone",
        );
    }
}

// Audience: HUMAN. Issue #34. Python test suites conventionally open
// with a module docstring, `from __future__ import annotations`,
// `import pytest`, `from typing import TYPE_CHECKING`, and an
// `if TYPE_CHECKING:` import block. That prologue is pure
// import/prologue boilerplate: it carries no semantic content a human
// would recognise as "copy-pasted code". Before the fix for #34 the
// prologue subtree survived the boilerplate filter (no
// `future_import_statement` carrier, no module-docstring carrier, and
// the `if_statement` wrapper around imports was not treated as an
// imports-only subtree), so deslop reported the prologue as a
// cross-file clone spanning every Python file in the repo. For a
// 40-file repo that produced a 109-member cluster; even a 6-file
// fixture reproduces the symptom.
#[test]
fn python_module_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture("python-prologue-false-positive");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "python prologue");
    Ok(())
}

// Audience: HUMAN. Issue #34, C# arm. The same prologue
// false-positive that hits Python `from __future__ import` /
// `if TYPE_CHECKING:` blocks also hits C# files: when many `.cs`
// files share the same `using ...;` block + `namespace X;` prologue
// but have entirely different class bodies, the sibling-window pass
// emits windows that span from the prologue into the class
// declaration. The `using_directive`/`file_scoped_namespace_declaration`
// k-grams dominate the window's token signature, so token Jaccard
// approaches 1.00 and LSH-only matching links every file into one
// cross-file cluster — even though the structural Merkle hashes
// disagree. The reported cluster from the user's repo had 109
// occurrences pinned at line 1, column 1 across the codebase; six
// distinct files reproduce the same shape here.
#[test]
fn csharp_using_namespace_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture("csharp-prologue-false-positive");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "csharp prologue");
    Ok(())
}

// Audience: HUMAN. Issue #34, Rust arm. Six Rust files share the same
// eight-line `use ...;` block but contain completely different items
// (a function, a struct, an async fetcher, a trait, a CSV parser, a
// retry policy). Sibling windows that begin on a `use_declaration`
// and extend into the next `function_item` / `struct_item` /
// `trait_item` carry token signatures dominated by
// `use_declaration __ident__` k-grams, pushing token Jaccard to 1.00.
// `use_declaration` is already a boilerplate carrier for subtree
// fingerprints, but the sibling-window emitter still produces windows
// that *start* inside the use block — and those windows do anchor
// cross-file LSH-only clusters. The fix must keep import scaffolding
// from anchoring cross-file matches in any language we parse.
#[test]
fn rust_use_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture("rust-prologue-false-positive");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "rust prologue");
    Ok(())
}

// Implements multi-language dispatch — three files routed by extension
// in one run.
#[test]
fn handles_mixed_language_fixture() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
fn default_output_written_to_current_directory() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .current_dir(tmp.path())
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .assert()
        .success();
    assert!(tmp.path().join("deslop-report.json").exists());
    assert!(tmp.path().join("deslop-report.txt").exists());
    assert!(tmp.path().join("deslop-report.html").exists());
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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
    seed_scan_root(&fixture("csharp-type3"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
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
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut first = Command::cargo_bin("deslop")?;
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
        .join(".deslop-cache")
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
    let mut second = Command::cargo_bin("deslop")?;
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
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let mut cmd = Command::cargo_bin("deslop")?;
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

// ===========================================================================
// OLLAMA-LIVE TESTS — require a running local Ollama daemon on
// 127.0.0.1:11434 with the `nomic-embed-text` model pulled.
// The `ollama_` name prefix is the marker: `make ci` filters them
// out via `cargo test ... -- --skip ollama_`; `make ci-ollama` runs
// them via `cargo test ollama_`. Every test below pins
// `--embedding-model nomic-embed-text` so assertions against
// `model_id` stay honest even if a developer's shell exports a
// different default. Reports are parsed via `serde_json` so the
// assertions are schema-aware rather than substring-guessing.
// ===========================================================================

/// Walks every cluster in `json` and returns the first whose
/// occurrences cover every file name in `required`. Used to pick
/// out the cross-file Type-4 cluster (Recursive.cs + Iterative.cs)
/// from the many within-file sibling-window clusters the fixture
/// also produces.
fn find_cross_file_cluster(
    json: &serde_json::Value,
    required: &[&str],
) -> Option<serde_json::Value> {
    let clusters = json.get("clusters")?.as_array()?;
    clusters
        .iter()
        .find(|cluster| {
            let Some(occurrences) = cluster.get("occurrences").and_then(|v| v.as_array()) else {
                return false;
            };
            let names: std::collections::HashSet<&str> = occurrences
                .iter()
                .filter_map(|occ| occ.get("path").and_then(|p| p.as_str()))
                .filter_map(|p| std::path::Path::new(p).file_name().and_then(|n| n.to_str()))
                .collect();
            required.iter().all(|needle| names.contains(needle))
        })
        .cloned()
}

/// Reads the JSON report at `path` and parses it into a
/// `serde_json::Value`. Tests assert against the parsed value so
/// trivial formatting changes in the renderer don't break them.
fn load_report_json(path: &Path) -> Result<serde_json::Value> {
    let raw = fs::read_to_string(path)?;
    let value = serde_json::from_str(&raw)?;
    Ok(value)
}

// Implements [FUSION-EMBED-PROVIDER] Type-4 end-to-end: the fixture
// pairs recursive and iterative implementations of factorial /
// fibonacci / sum-to-n. Without embeddings the two files share no
// structural or token signal. With live Ollama, the embedding pass
// must produce a *cross-file* cluster whose `embedding_cos > 0.3`
// and whose fused score preserves the strongest component while staying
// in the public confidence range.
#[test]
fn ollama_type4_cross_file_cluster_has_positive_embedding_signal() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-type4"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();
    let json = load_report_json(&out.json)?;
    let provenance = json
        .get("embedding_provenance")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("embedding_provenance missing or not an object"))?;
    assert_eq!(
        provenance.get("provider_id").and_then(|v| v.as_str()),
        Some("ollama"),
        "provider_id must pin to ollama: {provenance:?}",
    );
    assert_eq!(
        provenance.get("model_id").and_then(|v| v.as_str()),
        Some("nomic-embed-text"),
        "model_id must pin to nomic-embed-text: {provenance:?}",
    );
    let model_version = provenance
        .get("model_version")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        !model_version.is_empty(),
        "model_version must be non-empty so cache keys change on weight updates: {provenance:?}",
    );
    assert!(
        provenance
            .get("dimensions")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|d| d > 0),
        "dimensions must be positive: {provenance:?}",
    );
    let cluster =
        find_cross_file_cluster(&json, &["Recursive.cs", "Iterative.cs"]).ok_or_else(|| {
            anyhow::anyhow!("no cross-file cluster spanning Recursive.cs + Iterative.cs")
        })?;
    let signals = cluster
        .get("signals")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("cluster missing signals object"))?;
    let embedding_cos = signals
        .get("embedding_cos")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let token_jaccard = signals
        .get("token_jaccard")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let structural = signals
        .get("structural")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    let fused = signals
        .get("fused")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    assert!(
        embedding_cos > 0.3,
        "Type-4 cross-file cluster must carry a meaningful embedding_cos, got {embedding_cos}"
    );
    let deterministic_max = structural.max(token_jaccard);
    assert!(
        fused >= deterministic_max,
        "fused score {fused} must preserve the best deterministic signal {deterministic_max}",
    );
    assert!(
        fused >= embedding_cos,
        "fused score {fused} must preserve the embedding signal {embedding_cos}",
    );
    assert!(
        fused <= 1.0,
        "fused score {fused} must stay in the public confidence range",
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] auto mode: when Ollama is
// reachable, `--embeddings=auto` must silently upgrade to the live
// provider and record provenance. Complements
// `embeddings_auto_falls_back_when_provider_unreachable` which
// exercises the fallback direction against a dead endpoint.
#[test]
fn ollama_auto_mode_populates_provenance_when_reachable() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();
    let json = load_report_json(&out.json)?;
    let provenance = json
        .get("embedding_provenance")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            anyhow::anyhow!("auto mode with reachable Ollama must populate provenance")
        })?;
    assert_eq!(
        provenance.get("provider_id").and_then(|v| v.as_str()),
        Some("ollama"),
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] cache round-trip: the first
// run populates `.deslop-cache/embeddings/ollama/<model>/<version>/`
// with one `.bin` per fingerprint; the second run completes in a
// small fraction of the wall time because every embedding is
// served from disk. Each Ollama inference call is network-bound
// (tens of ms minimum), so a full re-embed of the fixture takes 30
// s or more. The 10 s cap catches cache misses without flaking.
#[test]
fn ollama_embedding_cache_persists_across_runs() -> Result<()> {
    use std::time::Instant;

    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-type4"), &scan_root)?;

    let mut first = Command::cargo_bin("deslop")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();

    let cache_root = scan_root
        .join(".deslop-cache")
        .join("embeddings")
        .join("ollama");
    let model_dir = fs::read_dir(&cache_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow::anyhow!("no model subdirectory under {}", cache_root.display()))?;
    let version_dir = fs::read_dir(&model_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow::anyhow!("no version subdirectory under {}", model_dir.display()))?;
    let cached_blob_count = fs::read_dir(&version_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "bin"))
        .count();
    assert!(
        cached_blob_count > 0,
        "first run must populate the embedding cache, found 0 .bin files in {}",
        version_dir.display(),
    );

    let started = Instant::now();
    let mut second = Command::cargo_bin("deslop")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "second run took {elapsed:?} — cache is not being used (cold Ollama runs take 30s+)",
    );

    let json = load_report_json(&tmp.path().join("second.json"))?;
    let provenance = json
        .get("embedding_provenance")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("second run lost provenance"))?;
    assert_eq!(
        provenance.get("model_id").and_then(|v| v.as_str()),
        Some("nomic-embed-text"),
    );
    Ok(())
}

// Implements the rendered-view contract: both text and HTML views
// must surface the Ollama provenance line so a human or agent
// reading the report knows which model produced the
// `embedding_cos` signals. JSON is canonical; this guards the
// derived views against silent drift.
#[test]
fn ollama_provenance_surfaces_in_text_and_html() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();
    let text = fs::read_to_string(&out.txt)?;
    assert!(
        text.contains("embeddings: ollama/nomic-embed-text@"),
        "text renderer must carry the Ollama provenance line: {text}"
    );
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("embeddings: ollama/nomic-embed-text@"),
        "html renderer must carry the Ollama provenance line: {html}"
    );
    Ok(())
}

// Implements [FUSION-EMBED-PROVIDER] × [PIPELINE-INCREMENTAL]: the
// two caches live side-by-side under `.deslop-cache/` and
// invalidate independently. The first run populates both
// (`fingerprints/...` and `embeddings/...`); the second run hits
// the fingerprint cache for every file AND reuses every embedding
// from disk, producing the same cross-file cluster as a cold run.
#[test]
fn ollama_incremental_plus_embeddings_second_run_hits_both_caches() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-type4"), &scan_root)?;

    let mut first = Command::cargo_bin("deslop")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();

    let first_json = load_report_json(&tmp.path().join("first.json"))?;
    let first_stats = first_json
        .get("cache_stats")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("first run missing cache_stats"))?;
    assert_eq!(
        first_stats.get("hits").and_then(serde_json::Value::as_u64),
        Some(0),
        "first incremental run must be a clean miss",
    );
    assert_eq!(
        first_stats
            .get("misses")
            .and_then(serde_json::Value::as_u64),
        Some(2),
        "first incremental run must register both files as misses",
    );

    let mut second = Command::cargo_bin("deslop")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("15")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .assert()
        .success();
    let second_json = load_report_json(&tmp.path().join("second.json"))?;
    let second_stats = second_json
        .get("cache_stats")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("second run missing cache_stats"))?;
    assert_eq!(
        second_stats.get("hits").and_then(serde_json::Value::as_u64),
        Some(2),
        "second run must hit the fingerprint cache for both files",
    );
    assert_eq!(
        second_stats
            .get("misses")
            .and_then(serde_json::Value::as_u64),
        Some(0),
        "second run must have zero fingerprint-cache misses",
    );

    let cluster = find_cross_file_cluster(&second_json, &["Recursive.cs", "Iterative.cs"])
        .ok_or_else(|| anyhow::anyhow!("cached run lost the cross-file cluster"))?;
    let embedding_cos = cluster
        .get("signals")
        .and_then(|s| s.get("embedding_cos"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default();
    assert!(
        embedding_cos > 0.3,
        "cached run must preserve the embedding_cos signal, got {embedding_cos}",
    );
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED]: `--output <nested-dir>/report`
// creates missing parent directories rather than erroring out.
#[test]
fn output_path_with_missing_parent_is_created() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let base = tmp.path().join("a").join("b").join("c").join("report");
    let mut cmd = Command::cargo_bin("deslop")?;
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
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults]\nreport_hide = [\"**/*.cs\"]\n")?;
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

// ===========================================================================
// P6 — incremental, perf, fixture-per-bug
// ===========================================================================

// Implements [PIPELINE-INCREMENTAL]: `--incremental` populates the
// fingerprint cache on the first pass and reports every file as a
// miss. A second run over the same unchanged tree must report every
// file as a hit and still surface the duplicated cluster.
#[test]
fn incremental_cache_hits_on_second_run() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
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
    let first_json = fs::read_to_string(tmp.path().join("first.json"))?;
    assert!(
        first_json.contains("\"hits\": 0"),
        "first run must be a clean miss: {first_json}"
    );
    assert!(
        first_json.contains("\"misses\": 2"),
        "first run must register two misses: {first_json}"
    );
    let cache_dir = scan_root.join(".deslop-cache").join("fingerprints");
    assert!(
        cache_dir.is_dir(),
        "fingerprint cache directory missing: {}",
        cache_dir.display()
    );
    let mut second = Command::cargo_bin("deslop")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .assert()
        .success();
    let second_json = fs::read_to_string(tmp.path().join("second.json"))?;
    assert!(
        second_json.contains("\"hits\": 2"),
        "second run must hit the cache for both files: {second_json}"
    );
    assert!(
        second_json.contains("\"misses\": 0"),
        "second run must have zero misses: {second_json}"
    );
    // Deduplication must still fire even when the fingerprints came
    // from the cache — the rehydration is only useful if downstream
    // clustering sees identical results.
    assert!(
        second_json.contains("\"structural\": 1.0"),
        "cached run must still detect the Type-2 cluster: {second_json}"
    );
    let second_txt = fs::read_to_string(tmp.path().join("second.txt"))?;
    assert!(
        second_txt.contains("cache: 2 hit / 0 miss"),
        "text renderer must surface cache stats: {second_txt}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] default-off: without
// `--incremental` the cache is neither read nor written. Stats read
// as a clean no-cache run (both counters zero) and no blobs land on
// disk — analysing a read-only checkout must never mutate it.
#[test]
fn default_run_skips_the_cache() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
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
        json.contains("\"hits\": 0"),
        "default run must record zero hits: {json}"
    );
    assert!(
        json.contains("\"misses\": 0"),
        "default run must not increment misses either: {json}"
    );
    assert!(
        !scan_root
            .join(".deslop-cache")
            .join("fingerprints")
            .exists(),
        "default run must not populate the fingerprint cache",
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] stale-blob recovery: a corrupt
// cache entry must be treated as a miss and overwritten, not surfaced
// as a hard error. The pipeline still produces a correct report.
#[test]
fn corrupt_cache_entry_degrades_to_miss() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
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
    let fingerprints_root = scan_root.join(".deslop-cache").join("fingerprints");
    for language_dir in fs::read_dir(&fingerprints_root)? {
        let language_path = language_dir?.path();
        for version_dir in fs::read_dir(&language_path)? {
            let version_path = version_dir?.path();
            for min_nodes_dir in fs::read_dir(&version_path)? {
                let min_nodes_path = min_nodes_dir?.path();
                for blob in fs::read_dir(&min_nodes_path)? {
                    let blob = blob?.path();
                    fs::write(&blob, b"not a valid cache blob")?;
                }
            }
        }
    }
    let mut second = Command::cargo_bin("deslop")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("second"))
        .assert()
        .success();
    let second_json = fs::read_to_string(tmp.path().join("second.json"))?;
    assert!(
        second_json.contains("\"misses\": 2"),
        "corrupt entries must be treated as misses: {second_json}"
    );
    assert!(
        second_json.contains("\"structural\": 1.0"),
        "analysis still produces the cluster after recovery: {second_json}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] help-text exposure: the
// `--incremental` opt-in must be documented so users can discover
// the cache without reading the source.
#[test]
fn help_text_documents_incremental_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--incremental"));
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] cache-write degradation: when
// the cache directory is read-only, the pipeline must log a warning
// and still produce a complete report. Exercises the
// `tracing::warn!(error, "fingerprint cache write failed")` branch
// that is otherwise only reachable on a genuine disk failure.
#[cfg(unix)]
#[test]
fn cache_write_failure_is_degraded_not_fatal() -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-small"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let locked_dir = scan_root
        .join(".deslop-cache")
        .join("fingerprints")
        .join("csharp")
        .join(env!("CARGO_PKG_VERSION"))
        .join("8");
    fs::create_dir_all(&locked_dir)?;
    let mut perms = fs::metadata(&locked_dir)?.permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&locked_dir, perms)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--incremental")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let mut restore = fs::metadata(&locked_dir)?.permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked_dir, restore)?;
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 2"),
        "pipeline must still report both files: {json}"
    );
    Ok(())
}

// Implements the P6 "perf pass" target in PLAN.md. The user-facing
// budget is <30 s on 100K-LOC C# with no embeddings. We can't assert
// on wallclock directly from `cargo test` (coverage instrumentation
// triples debug runtime), so this test exercises the pipeline on a
// modest synthetic C# corpus and asserts every file parsed + at
// least one cluster was ranked. The wallclock bound is deliberately
// lax — a pure regression guard against infinite loops or
// catastrophic quadratic explosions. The true SLA lives in
// [PERF-BUDGET-TYPE12] and is validated manually against a release
// binary on a real corpus.
#[test]
fn synthetic_corpus_scale_smoke_test() -> Result<()> {
    use std::time::Instant;

    let tmp = tempfile::tempdir()?;
    let corpus = tmp.path().join("corpus");
    fs::create_dir_all(&corpus)?;
    let files: u32 = 10;
    let methods_per_file: u32 = 10;
    for file_index in 0..files {
        let mut source = String::with_capacity(2048);
        source.push_str("namespace Generated;\npublic class Class");
        source.push_str(&file_index.to_string());
        source.push_str(" {\n");
        for method_index in 0..methods_per_file {
            let _ = writeln!(
                &mut source,
                "    public int Method{method_index}(int a, int b) {{ int x = a + b; int y = x * 2; if (y > 0) {{ return y; }} return x - a; }}",
            );
        }
        source.push_str("}\n");
        fs::write(corpus.join(format!("Class{file_index}.cs")), source)?;
    }
    let started = Instant::now();
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&corpus)
        .arg("--min-nodes")
        .arg("30")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let elapsed = started.elapsed();
    // 180 s is a regression guard, not a performance SLA. The real
    // budget is validated manually. A release-mode run against this
    // corpus takes well under a second.
    assert!(
        elapsed.as_secs() < 180,
        "synthetic corpus ran for {elapsed:?} — something is catastrophically wrong",
    );
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 10"),
        "synthetic corpus must analyse every generated file: {json}"
    );
    // Every file shares the identical method template, so the
    // ranked output must contain at least one cluster — catches
    // pipelines that silently drop everything.
    assert!(
        json.contains("\"weight\":"),
        "synthetic corpus produced no clusters: {json}"
    );
    Ok(())
}

// Implements the [BUG-FIXTURE] workflow from CLAUDE.md: every bug
// reproduced into `tests/fixtures/bug-*/` becomes a permanent e2e
// test. This is the seed example — an empty C# class body used to
// be silently dropped before the sibling-window fingerprint pass
// existed; the cluster test below pins that behaviour so the bug
// cannot regress.
#[test]
fn bug_fixture_walks_trivial_class_body_without_panicking() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("bug-empty-class"))
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 1"),
        "empty-class fixture must still analyse its one file: {json}"
    );
    Ok(())
}

// Implements [PIPELINE-NORMALIZE-AST] golden guard: `--debug-ast`
// on a hand-picked per-language fixture must match the committed
// expected dump byte-for-byte. Any drift in the grammar version,
// the `normalise_kind` match arms, or the child-ordering policy
// will trip this test — which is exactly what we want, because
// any of those changes silently alters the fingerprint and
// invalidates every user's cache.
//
// Each fixture exercises identifier collapse, literal collapse,
// comment drop, and the language-specific structural forms most
// likely to shift between grammar patch releases.
fn assert_ast_golden(fixture_dir: &str, sample_name: &str) -> Result<()> {
    let source = fixture(fixture_dir).join(sample_name);
    let expected_path = fixture(fixture_dir).join("Sample.expected.ast");
    let expected = fs::read_to_string(&expected_path)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let output = cmd
        .arg("--debug-ast")
        .arg(&source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let actual = String::from_utf8(output)?;
    assert_eq!(
        actual,
        expected,
        "AST dump drifted from {}. If this is intentional, regenerate with \
         `cargo run -q -- --debug-ast {}` and commit the updated .expected.ast.",
        expected_path.display(),
        source.display(),
    );
    Ok(())
}

#[test]
fn debug_ast_dump_matches_committed_golden() -> Result<()> {
    assert_ast_golden("ast-golden-csharp", "Sample.cs")
}

#[test]
fn debug_ast_dump_matches_committed_golden_rust() -> Result<()> {
    assert_ast_golden("ast-golden-rust", "Sample.rs")
}

#[test]
fn debug_ast_dump_matches_committed_golden_python() -> Result<()> {
    assert_ast_golden("ast-golden-python", "Sample.py")
}

// Implements [PIPELINE-NORMALIZE-AST] unsupported-extension: running
// `--debug-ast` on a file whose extension no parser claims must exit
// non-zero with a clear error, not panic or emit an empty dump.
#[test]
fn debug_ast_rejects_unknown_extension() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let path = tmp.path().join("sample.unknown");
    fs::write(&path, "// not a supported language\n")?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--debug-ast")
        .arg(&path)
        .assert()
        .failure()
        .stderr(contains("no language parser matches extension"));
    Ok(())
}

// Implements [PIPELINE-NORMALIZE-AST] help-text exposure: the
// `--debug-ast` developer flag must be documented.
#[test]
fn help_text_documents_debug_ast_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--debug-ast"));
    Ok(())
}

// ===========================================================================
// UX — timestamped-log-by-default, colored summary, console overrides
// ===========================================================================

// Implements [UX-LOG-FILE-DEFAULT]: a default run must write log
// events to a timestamped file next to the report and keep stderr
// clean of INFO-level log lines.
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
// friendly wording; this test guards the `Type-1/2` string the
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
        stderr.contains("Type-1/2"),
        "--technical must print the Type-taxonomy breakdown: {stderr}"
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
        "empty scan must not print a worst-offender line: {stderr}"
    );
    Ok(())
}

// -----------------------------------------------------------------
// P6.2 — Repo-wide duplication metric + fail-over threshold
// Implements [METRICS-REPO] and [EXIT-CODES].
// -----------------------------------------------------------------

/// Writes two C# files whose classes are Type-2 clones of each other,
/// rooted at `dir`. Returns the hand-countable line total: each file is
/// 12 physical lines, so the pair contributes 24 analysed LOC.
fn write_clone_pair(dir: &Path) -> Result<u64> {
    fs::create_dir_all(dir)?;
    let alpha = "namespace Alpha\n\
                 {\n\
                 public class Processor\n\
                 {\n\
                 public int Compute(int input)\n\
                 {\n\
                 if (input < 0) { return 0; }\n\
                 int total = 0;\n\
                 for (int i = 0; i < input; i = i + 1) { total = total + i; }\n\
                 return total;\n\
                 }\n\
                 }\n\
                 }\n";
    let beta = "namespace Beta\n\
                {\n\
                public class Summer\n\
                {\n\
                public int Run(int limit)\n\
                {\n\
                if (limit < 0) { return 0; }\n\
                int acc = 0;\n\
                for (int j = 0; j < limit; j = j + 1) { acc = acc + j; }\n\
                return acc;\n\
                }\n\
                }\n\
                }\n";
    fs::write(dir.join("Alpha.cs"), alpha)?;
    fs::write(dir.join("Beta.cs"), beta)?;
    // Each file = 13 newline-terminated lines. Two files => 26.
    Ok(26)
}

/// Returns the parsed JSON report from a successful run.
fn read_json_report(path: &Path) -> Result<serde_json::Value> {
    let body = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&body)?)
}

/// Looks up a named field on `value`; returns `Value::Null` when the
/// field is absent so callers get a deterministic `!=` instead of a
/// panic ([TESTS-NO-INDEXING]).
fn field<'a>(value: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    value.get(name).unwrap_or(&serde_json::Value::Null)
}

/// Shortcut for `field(field(value, "metrics"), key)`.
fn metric_field<'a>(report: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    field(field(report, "metrics"), key)
}

/// Shortcut for `field(field(field(value, "metrics"), "threshold"), key)`.
fn threshold_field<'a>(report: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    field(metric_field(report, "threshold"), key)
}

// Implements [METRICS-REPO]: empty corpus yields zero metrics. Still a
// valid report with schema v3, still serialises the metrics block.
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

// Implements [EXIT-CODES]: --fail-over 100.0 is never breached; exit 0.
#[test]
fn fail_over_cli_passes_under_threshold() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--fail-over")
        .arg("100")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[threshold]\nmax_duplication_percent = 0.0\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[threshold]\nmax_duplication_percent = 0.0\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--fail-over")
        .arg("100")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[threshold]\nmax_duplication_percent = 0.0\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--no-fail-over")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--fail-over")
        .arg("-1.0")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .code(2);
    Ok(())
}

// Implements [METRICS-REPO] + [OUTPUT-SCHEMA-JSON]: `--from-report`
// replays a v3 report, including its metrics block, without re-running
// the pipeline. Applied `--fail-over` on the replay beats any earlier
// threshold.
#[test]
fn from_report_replays_metrics_without_reanalysing() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    let initial = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let original = read_json_report(&initial.json)?;
    let original_metrics = field(&original, "metrics").clone();
    // Replay: write into a second output prefix so we don't clobber
    // the source JSON, and re-render from the first.
    let replay_prefix = tmp.path().join("replay");
    let mut cmd2 = Command::cargo_bin("deslop")?;
    let _assertion2 = cmd2
        .arg(&scan_root)
        .arg("--from-report")
        .arg(&initial.json)
        .arg("--no-color")
        .arg("--output")
        .arg(&replay_prefix)
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
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
    let html_breached = fs::read_to_string(&out.html)?;
    assert!(
        html_breached.contains("metrics-banner--breached"),
        "breached HTML must carry the breached class"
    );

    // Neutral variant (no threshold).
    let tmp2 = tempfile::tempdir()?;
    let scan_root2 = tmp2.path().join("src");
    let _ = write_clone_pair(&scan_root2)?;
    let out2 = outputs_under(tmp2.path());
    let mut cmd2 = Command::cargo_bin("deslop")?;
    let _assertion2 = cmd2
        .arg(&scan_root2)
        .arg("--min-nodes")
        .arg("8")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp2.path().join("report"))
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
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--fail-over")
        .arg("150.0")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .code(2);
    Ok(())
}

// Implements [EXIT-CODES]: `--fail-over NaN` is not finite and exits 2.
#[test]
fn fail_over_nan_exits_two() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--fail-over")
        .arg("NaN")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .code(2);
    Ok(())
}

// Implements [EXIT-CODES]: an invalid threshold in `.deslop.toml`
// propagates as exit 1 (runtime error) with the offending path in the
// diagnostic. `max_duplication_percent = 150` is out of range.
#[test]
fn config_threshold_out_of_range_fails_runtime() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let _ = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[threshold]\nmax_duplication_percent = 150.0\n",
    )?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--no-color")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .code(1);
    Ok(())
}

// Implements [METRICS-REPO]: `RepoMetrics::default()` and `empty()`
// deserialise as zero metrics through `--from-report` when reading an
// older (v2) report that pre-dates the field.
#[test]
fn from_report_rehydrates_missing_metrics_as_zero() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    // Minimal report missing `metrics` and `cluster.bucket`.
    // `#[serde(default)]` keeps older reports round-tripping while
    // `--from-report` rehydrates the bucket from the signal triple.
    let v2 = "{\n\
              \"report_schema_version\": 1,\n\
              \"tool_version\": \"legacy\",\n\
              \"min_nodes\": 30,\n\
              \"files_analysed\": 0,\n\
              \"clusters_hidden\": 0,\n\
              \"schema_doc\": \"\",\n\
              \"action_hints\": [],\n\
              \"embedding_provenance\": null,\n\
              \"clusters\": [{\n\
                \"id\": \"abc123\",\n\
                \"weight\": 1.0,\n\
                \"size\": 2,\n\
                \"canonical_node_count\": 8,\n\
                \"signals\": {\"structural\": 1.0, \"token_jaccard\": 1.0, \"embedding_cos\": 0.0, \"fused\": 1.0},\n\
                \"occurrences\": [],\n\
                \"summary\": \"legacy\",\n\
                \"interpretation\": \"legacy\"\n\
              }]\n\
              }\n";
    let legacy_path = tmp.path().join("legacy.json");
    fs::write(&legacy_path, v2)?;
    let output_prefix = tmp.path().join("report");
    let out = outputs_under(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(tmp.path())
        .arg("--from-report")
        .arg(&legacy_path)
        .arg("--no-color")
        .arg("--output")
        .arg(&output_prefix)
        .assert()
        .success();
    let json = read_json_report(&out.json)?;
    assert_eq!(metric_field(&json, "analysed_loc").as_u64(), Some(0));
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    let bucket = json
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .and_then(|clusters| clusters.first())
        .and_then(|cluster| cluster.get("bucket"))
        .and_then(serde_json::Value::as_str);
    assert_eq!(bucket, Some("identical"));
    Ok(())
}
