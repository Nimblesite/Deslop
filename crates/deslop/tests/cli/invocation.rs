use crate::support::*;
use std::fmt::Write as _;

#[test]
fn prints_version_and_exits_zero() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let expected = format!("deslop {}\n", expected_version());
    let _assertion = cmd
        .arg("--version")
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let output = Command::cargo_bin("deslop")?
        .args(["--version", "--json"])
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_version_manifest(&value, "deslop", "cli");
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    deslop_test_support::assert_version_manifest(value, name, kind, expected_version());
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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
    let mut cmd = deslop_command(tmp.path(), &tmp.path().join("report"))?;
    let _assertion = cmd.assert().success();
    assert!(out.json.exists(), "json missing at {}", out.json.display());
    assert!(out.txt.exists(), "txt missing at {}", out.txt.display());
    assert!(out.html.exists(), "html missing at {}", out.html.display());
    Ok(())
}

// Implements [OUTPUT-FORMAT-DERIVED]: the default run emits JSON, text,
// and HTML side by side. All three must carry the current report fields.
#[test]
fn default_run_emits_all_three_formats() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let json = fs::read_to_string(&out.json)?;
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
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
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
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--nojson", "--nohtml"])
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
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--nojson", "--notext", "--nohtml"])
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
    let mut first = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = first
        .args(["--min-nodes", "8", "--notext", "--nohtml"])
        .assert()
        .success();
    assert!(out.json.exists());
    let rendered_dir = tempfile::tempdir()?;
    let rerender = outputs_under(rendered_dir.path());
    let mut second = deslop_command(tmp.path(), &rendered_dir.path().join("report"))?;
    let _assertion = second
        .arg("--from-report")
        .arg(&out.json)
        .arg("--nojson")
        .assert()
        .success();
    assert!(rerender.txt.exists(), "txt not re-rendered");
    assert!(rerender.html.exists(), "html not re-rendered");
    Ok(())
}

// Implements [PIPELINE-CLUSTER-EXACT] + [PIPELINE-NORMALIZE-AST]: two
// C# files with the same structure but renamed identifiers (Type-2
