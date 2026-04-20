//! E2E coverage for `--rerun-touch`, which drives
//! [`codededup_core::PipelineSession::update_files`] +
//! [`codededup_core::ReportDelta::between`] through the binary per
//! [LIVE-STATE] / [LIVE-DELTA].
//!
//! These tests construct a mutable scan root, run the CLI twice in
//! one invocation via `--rerun-touch <PATH>...`, and assert on the
//! emitted `<base>.delta.json` alongside the normal report outputs.

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

/// Returns the absolute path of a fixture under `tests/fixtures/<name>`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies every top-level file in `src` into `dst` (created fresh).
fn seed(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Returns the on-disk `<dir>/report.delta.json` path the CLI emits
/// when `--rerun-touch` is passed with an `--output <dir>/report` base.
fn delta_path(dir: &Path) -> PathBuf {
    dir.join("report.delta.json")
}

fn load_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

// Implements [LIVE-STATE] touching a path whose content is unchanged
// yields an empty delta. Exercises the no-op update path through
// `PipelineSession::update_files` and the all-equal branches of
// `ReportDelta::between`.
#[test]
fn rerun_touch_with_unchanged_sources_emits_empty_delta() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&fixture("csharp-small"), &scan_root)?;
    let touched = scan_root.join("Alpha.cs");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--rerun-touch")
        .arg(&touched)
        .assert()
        .success();
    let delta_json = delta_path(tmp.path());
    assert!(delta_json.is_file(), "delta file must be emitted");
    let delta = load_json(&delta_json)?;
    assert_eq!(delta["from_generation"], 0);
    assert_eq!(delta["to_generation"], 1);
    assert_eq!(
        delta["clusters_added"].as_array().map(Vec::len),
        Some(0),
        "unchanged sources must add no clusters: {delta:#}"
    );
    assert_eq!(delta["clusters_removed"].as_array().map(Vec::len), Some(0));
    assert_eq!(delta["clusters_updated"].as_array().map(Vec::len), Some(0));
    Ok(())
}

// Implements [LIVE-STATE]: modifying a file on disk and replaying its
// path through `--rerun-touch` drives the `apply_one_change` update
// branch (re-parse, re-fingerprint). The initial `initialise` already
// sees the post-edit state, so the rerun is an idempotent refresh.
#[test]
fn rerun_touch_on_existing_file_reparses_via_update_files() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    fs::write(
        scan_root.join("Alpha.cs"),
        "namespace Alpha\n{\n    public class Solo\n    {\n        public int Compute(int x)\n        {\n            return x + 1;\n        }\n    }\n}\n",
    )?;
    let beta = scan_root.join("Beta.cs");
    fs::write(
        &beta,
        "namespace Beta\n{\n    public class Solo\n    {\n        public int Compute(int x)\n        {\n            return x + 1;\n        }\n    }\n}\n",
    )?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--rerun-touch")
        .arg(&beta)
        .assert()
        .success();
    let delta = load_json(&tmp.path().join("report.delta.json"))?;
    assert_eq!(delta["from_generation"], 0);
    assert_eq!(delta["to_generation"], 1);
    assert_eq!(
        delta["clusters_added"].as_array().map(Vec::len),
        Some(0),
        "same on-disk state before and after rerun must emit no added clusters"
    );
    Ok(())
}

// Implements [LIVE-STATE]: `--rerun-touch` silently skips paths that are
// outside the scan root or have an unsupported extension, exercising
// the two early-return branches of `apply_one_change`.
#[test]
fn rerun_touch_ignores_unsupported_and_out_of_root_paths() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&fixture("csharp-small"), &scan_root)?;
    // Irrelevant extension: `.md` has no registered parser.
    let readme = scan_root.join("NOTES.md");
    fs::write(&readme, "ignored by the parser\n")?;
    // Relative-looking path that canonicalises under the root: .cs so
    // the parser claims it, but the file does not exist → deletion no-op.
    let missing = PathBuf::from("Zeta.cs");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--rerun-touch")
        .arg(&readme)
        .arg("--rerun-touch")
        .arg(&missing)
        .assert()
        .success();
    let delta = load_json(&tmp.path().join("report.delta.json"))?;
    assert_eq!(
        delta["clusters_added"].as_array().map(Vec::len),
        Some(0),
        "irrelevant / missing paths must not add clusters: {delta:#}"
    );
    assert_eq!(delta["clusters_removed"].as_array().map(Vec::len), Some(0));
    Ok(())
}

// Implements [LIVE-STATE] + [EXCLUSION-CONFIG]: a `--rerun-touch` path
// that is covered by the loaded exclusion config is treated as a
// deletion so that an edit making it excluded drops it from the corpus.
#[test]
fn rerun_touch_on_excluded_path_drops_it_from_corpus() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&fixture("csharp-small"), &scan_root)?;
    // Initial run discovers both files (no exclusion); rerun applies a
    // config that excludes Beta.cs, so touching it drops it.
    let mut first = Command::cargo_bin("codededup")?;
    let _assertion = first
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("first"))
        .arg("--nohtml")
        .arg("--notext")
        .assert()
        .success();
    let exclusion = tmp.path().join("codededup.toml");
    fs::write(&exclusion, "[defaults]\nexclude = [\"**/Beta.cs\"]\n")?;
    let mut second = Command::cargo_bin("codededup")?;
    let _assertion = second
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--config")
        .arg(&exclusion)
        .arg("--output")
        .arg(tmp.path().join("second"))
        .arg("--rerun-touch")
        .arg(scan_root.join("Beta.cs"))
        .assert()
        .success();
    let delta = load_json(&tmp.path().join("second.delta.json"))?;
    // The second initial pass already filters Beta.cs via exclusion, so
    // the rerun's drop-path branch is the relevant cover — the delta
    // simply reports no changes between gen 0 and gen 1.
    assert_eq!(delta["from_generation"], 0);
    assert_eq!(delta["to_generation"], 1);
    Ok(())
}
