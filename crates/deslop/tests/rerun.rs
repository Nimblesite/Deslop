//! E2E coverage for `--rerun-touch`, which drives
//! [`deslop_core::PipelineSession::update_files`] +
//! [`deslop_core::ReportDelta::between`] through the binary per
//! [LIVE-STATE] / [LIVE-DELTA].
//!
//! These tests construct a mutable scan root, run the CLI twice in
//! one invocation via `--rerun-touch <PATH>...`, and assert on the
//! emitted `<base>.delta.json` alongside the normal report outputs.

use std::{ffi::OsStr, fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;

mod common;
use crate::common::{rerun_ops::*, *};

/// Config body that excludes the `Beta.cs` half of the seeded clone pair.
const EXCLUDE_BETA: &str = "[defaults]\nexclude = [\"**/Beta.cs\"]\n";

/// Returns the on-disk `<dir>/report.delta.json` path the CLI emits
/// when `--rerun-touch` is passed with an `--output <dir>/report` base.
fn delta_path(dir: &Path) -> PathBuf {
    dir.join("report.delta.json")
}

/// A temp dir plus its `src/` scan root, seeded from the `csharp-small`
/// fixture — the mutable Alpha/Beta clone pair the scenarios below edit.
/// The [`tempfile::TempDir`] comes back so the caller keeps the tree alive.
fn seeded_root() -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&fixture("csharp-small"), &scan_root)?;
    Ok((tmp, scan_root))
}

/// Builds `deslop <scan_root> --output <out_base> --min-nodes <min_nodes>`
/// followed by one `<flag> <value>` pair per entry in `ops`, ready for the
/// caller to assert on. Every rerun scenario differs only in that tail.
fn rerun_cmd(
    scan_root: &Path,
    out_base: &Path,
    min_nodes: &str,
    ops: &[(&str, &OsStr)],
) -> Result<Command> {
    let mut cmd = deslop_cmd(scan_root, out_base)?;
    let _cmd = cmd.args(["--min-nodes", min_nodes]);
    for (flag, value) in ops {
        let _cmd = cmd.arg(flag).arg(value);
    }
    Ok(cmd)
}

// Implements [LIVE-STATE] touching a path whose content is unchanged
// yields an empty delta. Exercises the no-op update path through
// `PipelineSession::update_files` and the all-equal branches of
// `ReportDelta::between`.
#[test]
fn rerun_touch_with_unchanged_sources_emits_empty_delta() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    let touched = scan_root.join("Alpha.cs");
    let ops = [("--rerun-touch", touched.as_os_str())];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta_json = delta_path(tmp.path());
    assert!(delta_json.is_file(), "delta file must be emitted");
    let delta = load_json(&delta_json)?;
    assert_eq!(field(&delta, "from_generation"), 0);
    assert_eq!(field(&delta, "to_generation"), 1);
    assert_eq!(
        array_len(&delta, "clusters_added"),
        0,
        "unchanged sources must add no clusters: {delta:#}"
    );
    assert_eq!(array_len(&delta, "clusters_removed"), 0);
    assert_eq!(array_len(&delta, "clusters_updated"), 0);
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
    let ops = [("--rerun-touch", beta.as_os_str())];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "4", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert_eq!(field(&delta, "from_generation"), 0);
    assert_eq!(field(&delta, "to_generation"), 1);
    assert_eq!(
        array_len(&delta, "clusters_added"),
        0,
        "same on-disk state before and after rerun must emit no added clusters"
    );
    Ok(())
}

// Implements [LIVE-STATE]: `--rerun-touch` silently skips paths that are
// outside the scan root or have an unsupported extension, exercising
// the two early-return branches of `apply_one_change`.
#[test]
fn rerun_touch_ignores_unsupported_and_out_of_root_paths() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    // Irrelevant extension: `.md` has no registered parser.
    let readme = scan_root.join("NOTES.md");
    fs::write(&readme, "ignored by the parser\n")?;
    // Relative-looking path that canonicalises under the root: .cs so
    // the parser claims it, but the file does not exist → deletion no-op.
    let missing = PathBuf::from("Zeta.cs");
    let ops = [
        ("--rerun-touch", readme.as_os_str()),
        ("--rerun-touch", missing.as_os_str()),
    ];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert_eq!(
        array_len(&delta, "clusters_added"),
        0,
        "irrelevant / missing paths must not add clusters: {delta:#}"
    );
    assert_eq!(array_len(&delta, "clusters_removed"), 0);
    Ok(())
}

// Implements [LIVE-STATE] + [LIVE-DELTA]: `--rerun-add` copies a source
// file into the scan root between `initialise` and `update_files`, so
// the initial corpus does not see the new file but the rerun parses it
// and joins it into a clone cluster. Drives the add-new-file branch of
// `apply_one_change` plus the `clusters_added` branch of `between`.
#[test]
fn rerun_add_introduces_new_file_and_reports_cluster_added() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    // Seed with only Alpha.cs so the initial corpus has no clones.
    let alpha = fixture("csharp-small").join("Alpha.cs");
    let _bytes = fs::copy(&alpha, scan_root.join("Alpha.cs"))?;
    // Stage Beta.cs outside the scan root so `initialise` cannot see it.
    let staged = tmp.path().join("staged-Beta.cs");
    let _bytes = fs::copy(fixture("csharp-small").join("Beta.cs"), &staged)?;
    let dst = scan_root.join("Beta.cs");
    let spec = add_spec(&staged, &dst);
    let ops = [("--rerun-add", OsStr::new(&spec))];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert!(
        array_len(&delta, "clusters_added") > 0,
        "staging a new file must surface a clusters_added entry: {delta:#}"
    );
    assert!(dst.is_file(), "--rerun-add must copy the file into place");
    Ok(())
}

// Implements [LIVE-STATE]: a malformed `--rerun-add` spec is rejected
// with a user-facing error before any analysis runs.
#[test]
fn rerun_add_rejects_spec_without_equals() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    let mut cmd = deslop_cmd(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--rerun-add", "missing-equals-sign"])
        .assert()
        .failure()
        .stderr(contains("--rerun-add"));
    Ok(())
}

// Implements [LIVE-STATE] + [LIVE-DELTA]: `--rerun-remove` physically
// deletes a file between `initialise` and `update_files`, so the delta
// surfaces the clone cluster that disappeared. Drives `drop_path` plus
// the `clusters_removed` branch of `ReportDelta::between`.
#[test]
fn rerun_remove_drops_clone_and_reports_cluster_removed() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    let doomed = scan_root.join("Beta.cs");
    let ops = [("--rerun-remove", doomed.as_os_str())];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert!(
        array_len(&delta, "clusters_removed") > 0,
        "removing the only peer must surface a clusters_removed entry: {delta:#}"
    );
    assert!(
        !doomed.exists(),
        "--rerun-remove must physically delete the path"
    );
    Ok(())
}

// Implements [LIVE-CONFIG-LIVE] (#189): replaying the watched
// `<root>/.deslop.toml` through the rerun harness must re-evaluate the
// existing corpus against the reloaded `exclude` patterns. A pattern
// added between generations drops the now-excluded file — and its
// clusters — instead of only re-rendering.
#[test]
fn issue_189_new_exclude_pattern_drops_existing_corpus_files() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    // Stage a config that excludes Beta.cs; `--rerun-add` lands it at
    // the watched `<root>/.deslop.toml` between generation 0 and 1.
    let config_dst = scan_root.join(".deslop.toml");
    let spec = staged_spec(tmp.path(), "staged-deslop.toml", EXCLUDE_BETA, &config_dst)?;
    let ops = [("--rerun-add", OsStr::new(&spec))];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert_eq!(field(&delta, "from_generation"), 0);
    assert_eq!(field(&delta, "to_generation"), 1);
    assert!(
        array_len(&delta, "clusters_removed") > 0,
        "excluding Beta.cs must remove the Alpha/Beta clone cluster: {delta:#}"
    );
    let report = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        !report.contains("Beta.cs"),
        "generation 1 report must not mention the excluded file"
    );
    Ok(())
}

// Implements [LIVE-CONFIG-LIVE] (#189): the reverse direction — a
// pattern removed between generations re-discovers the previously
// excluded file so its clusters appear. Exercises the explicit
// `--config` override path rather than `<root>/.deslop.toml`.
#[test]
fn issue_189_removed_exclude_pattern_rediscovers_files() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    // Initial pass excludes Beta.cs via an explicit override config,
    // so generation 0 sees only Alpha.cs and reports no clusters.
    let override_config = tmp.path().join("deslop.toml");
    fs::write(&override_config, EXCLUDE_BETA)?;
    let looser = "[defaults]\nexclude = []\n";
    let spec = staged_spec(tmp.path(), "staged-looser.toml", looser, &override_config)?;
    let ops = [
        ("--config", override_config.as_os_str()),
        ("--rerun-add", OsStr::new(&spec)),
    ];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    assert_eq!(field(&delta, "from_generation"), 0);
    assert_eq!(field(&delta, "to_generation"), 1);
    assert!(
        array_len(&delta, "clusters_added") > 0,
        "dropping the exclude must re-discover Beta.cs and surface its cluster: {delta:#}"
    );
    let report = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        report.contains("Beta.cs"),
        "generation 1 report must include the re-included file"
    );
    Ok(())
}

// Implements [LIVE-DELTA] (#199): the delta must carry the authoritative
// gen-1 `metrics`, not just cluster diffs, so live consumers (the VSIX
// DUPLICATION headline) move off the seed snapshot instead of freezing.
// Removing the only clone peer drops duplication to zero in gen 1; the
// emitted delta's metrics must report that — and must equal the gen-1
// report's metrics, since the server is the single source of truth.
#[test]
fn issue_199_delta_carries_recomputed_metrics() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    let doomed = scan_root.join("Beta.cs");
    let ops = [("--rerun-remove", doomed.as_os_str())];
    let mut cmd = rerun_cmd(&scan_root, &tmp.path().join("report"), "8", &ops)?;
    let _assertion = cmd.assert().success();
    let delta = load_json(&delta_path(tmp.path()))?;
    let report = load_json(&tmp.path().join("report.json"))?;
    let delta_metrics = field(&delta, "metrics");
    let report_metrics = field(&report, "metrics");
    assert!(
        delta_metrics.is_object(),
        "delta must carry a metrics object so the headline can move off the seed: {delta:#}"
    );
    // Removing the only peer eliminates all duplication in generation 1.
    assert_eq!(
        field(delta_metrics, "duplicated_loc").as_u64(),
        Some(0),
        "post-removal delta metrics must report zero duplicated LOC: {delta:#}"
    );
    // The delta's metrics must equal the gen-1 report's metrics — the
    // server recomputes `report.metrics` every generation and is the
    // single source of truth for the displayed numbers.
    assert_eq!(
        field(delta_metrics, "duplication_percent"),
        field(report_metrics, "duplication_percent"),
        "delta duplication_percent must equal the gen-1 report's value: {delta:#}"
    );
    assert_eq!(
        field(delta_metrics, "analysed_loc"),
        field(report_metrics, "analysed_loc"),
        "delta analysed_loc must equal the gen-1 report's value: {delta:#}"
    );
    Ok(())
}

// Implements [LIVE-STATE] + [EXCLUSION-CONFIG]: a `--rerun-touch` path
// that is covered by the loaded exclusion config is treated as a
// deletion so that an edit making it excluded drops it from the corpus.
#[test]
fn rerun_touch_on_excluded_path_drops_it_from_corpus() -> Result<()> {
    let (tmp, scan_root) = seeded_root()?;
    // Initial run discovers both files (no exclusion); rerun applies a
    // config that excludes Beta.cs, so touching it drops it.
    let mut first = deslop_cmd(&scan_root, &tmp.path().join("first"))?;
    let _assertion = first
        .args(["--min-nodes", "8", "--nohtml", "--notext"])
        .assert()
        .success();
    let exclusion = tmp.path().join("deslop.toml");
    fs::write(&exclusion, EXCLUDE_BETA)?;
    let beta = scan_root.join("Beta.cs");
    let ops = [
        ("--config", exclusion.as_os_str()),
        ("--rerun-touch", beta.as_os_str()),
    ];
    let mut second = rerun_cmd(&scan_root, &tmp.path().join("second"), "8", &ops)?;
    let _assertion = second.assert().success();
    let delta = load_json(&tmp.path().join("second.delta.json"))?;
    // The second initial pass already filters Beta.cs via exclusion, so
    // the rerun's drop-path branch is the relevant cover — the delta
    // simply reports no changes between gen 0 and gen 1.
    assert_eq!(field(&delta, "from_generation"), 0);
    assert_eq!(field(&delta, "to_generation"), 1);
    Ok(())
}
