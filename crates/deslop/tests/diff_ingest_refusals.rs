//! Diff-ingest refusal end-to-end ([CLI-ARG-DIFF],
//! [PIPELINE-DIFF-INGEST], [METRICS-DIFF-SCOPE]): the three P0
//! fail-open paths from the #364 branch review. A malformed section
//! without a `+++` target, a supported in-root target missing from the
//! tree, and a metadata-only 100%-similarity git copy must all be
//! surfaced — refused with exit 2 naming the offence, or (for the
//! copy) counted as wholesale added duplication that breaches
//! `--fail-over 0` — never silently accepted as an empty scope.
//!
//! Every corpus is built in a tempdir at runtime: the working
//! directory is the tempdir root, the scan root is its `repo/`
//! subdirectory, and diffs arrive on stdin via `--diff -` with
//! repo-root-relative `a/` / `b/` paths, mirroring the CI flow.

mod common;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use assert_cmd::{assert::Assert, Command};
use serde_json::Value;

use crate::common::{clusters, field, load_json, occurrences, Result};

/// A ten-line function duplicated byte-for-byte across the legacy
/// pair — enough structure to cluster in the `identical` bucket.
const DUP_SOURCE: &str = concat!(
    "pub fn fold_digest(values: &[u32]) -> u32 {\n",
    "    let mut acc = 11_u32;\n",
    "    for value in values {\n",
    "        acc = acc.rotate_left(5).wrapping_add(*value);\n",
    "        if acc % 7 == 0 {\n",
    "            acc = acc.wrapping_mul(3);\n",
    "        }\n",
    "    }\n",
    "    acc ^ 0x2f2f\n",
    "}\n",
);

/// Line count of [`DUP_SOURCE`], the exact `added_loc` a wholesale
/// copy of one file must contribute.
const DUP_LINES: u64 = 10;

/// Writes `files` (as `repo/`-relative path → content) under
/// `<root>/repo` and returns the tempdir holding them.
fn build_repo(files: &[(&str, &str)]) -> Result<tempfile::TempDir> {
    let tmp = tempfile::tempdir()?;
    for (path, content) in files {
        let absolute = tmp.path().join("repo").join(path);
        let parent = absolute.parent().context("file path has a parent")?;
        fs::create_dir_all(parent)?;
        fs::write(absolute, content)?;
    }
    Ok(tmp)
}

/// The byte-identical `dup_a.rs` / `dup_b.rs` pair every scenario
/// starts from.
fn dup_pair_repo() -> Result<tempfile::TempDir> {
    build_repo(&[("src/dup_a.rs", DUP_SOURCE), ("src/dup_b.rs", DUP_SOURCE)])
}

/// Runs `deslop repo --output <output> --no-incremental --embeddings
/// off --diff - <extra...>` from `cwd`, feeding `diff` on stdin.
fn run_diff(cwd: &Path, output: &Path, diff: &str, extra: &[&str]) -> Result<Assert> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _configured = cmd
        .current_dir(cwd)
        .arg("repo")
        .arg("--output")
        .arg(output)
        .arg("--no-incremental")
        .args(["--embeddings", "off", "--diff", "-"])
        .args(extra)
        .write_stdin(diff);
    Ok(cmd.assert())
}

/// A fresh output prefix inside its own tempdir.
fn output_prefix() -> Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    Ok((tmp, output))
}

/// The process stderr of a finished assertion.
fn stderr_of(assert: &Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// The `metrics.diff` block of a report.
fn diff_metrics(report: &Value) -> Value {
    field(field(report, "metrics"), "diff").clone()
}

/// Asserts the zero-scope report shape shared by every "stays
/// ignorable" scenario (exit 0 is asserted by the caller's `.code(0)`):
/// no added lines, no surviving clusters, and the untouched legacy
/// cluster omitted-but-counted.
fn assert_zero_scope_pass(output: &Path) -> Result<()> {
    let report = load_json(&output.with_extension("json"))?;
    let diff = diff_metrics(&report);
    assert_eq!(field(&diff, "added_loc"), 0, "no in-scope lines: {report:#}");
    assert_eq!(field(&diff, "duplicated_added_loc"), 0);
    assert_eq!(field(&diff, "duplication_percent"), 0.0);
    assert_eq!(field(field(&diff, "threshold"), "breached"), false);
    assert_eq!(clusters(&report).len(), 0, "nothing intersects the diff");
    assert_eq!(
        field(&report, "clusters_outside_diff"),
        1,
        "the untouched dup pair is omitted, not lost: {report:#}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-1: `diff nonsense` + a hunk used to parse
// as a pathless section the verifier ignored — exit 0, zero added
// LOC, every legacy cluster omitted under `--only-changed`. A hunk
// header in a section that never saw `+++` must be a parse refusal
// (usage error, exit 2) naming the offending diff line.
#[test]
fn hunk_without_a_target_line_is_refused_naming_the_line() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let assert = run_diff(
        repo.path(),
        &output,
        "diff nonsense\n@@ -0,0 +1 @@\n+x\n",
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(2);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("invalid unified diff"),
        "the refusal is a parse error: {stderr}"
    );
    assert!(
        stderr.contains("line 2"),
        "the refusal names the hunk header's diff line: {stderr}"
    );
    assert!(
        stderr.contains("+++"),
        "the refusal names the missing target line: {stderr}"
    );
    assert!(
        !output.with_extension("json").exists(),
        "a refused diff must not produce a report"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-1 contrast: legitimate hunkless or
// targetless sections — binary entries, pure rename metadata, and
// deletions (`+++ /dev/null` is a seen target) — must keep parsing
// and ingesting as an empty scope, not become refusals.
#[test]
fn legitimate_targetless_sections_stay_ingestible() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let diff = concat!(
        "diff --git a/repo/logo.png b/repo/logo.png\n",
        "index 1111111..2222222 100644\n",
        "Binary files a/repo/logo.png and b/repo/logo.png differ\n",
        "diff --git a/repo/src/old.rs b/repo/src/new.rs\n",
        "similarity index 100%\n",
        "rename from repo/src/old.rs\n",
        "rename to repo/src/new.rs\n",
        "diff --git a/repo/src/gone.rs b/repo/src/gone.rs\n",
        "deleted file mode 100644\n",
        "--- a/repo/src/gone.rs\n",
        "+++ /dev/null\n",
        "@@ -1 +0,0 @@\n",
        "-fn gone() {}\n",
    );
    let _assert = run_diff(
        repo.path(),
        &output,
        diff,
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(0);
    assert_zero_scope_pass(&output)?;
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2: a valid new-file hunk for a supported
// file inside the scan root that exists neither in the corpus nor on
// disk used to be silently ignored — exit 0 with zero scope. It must
// be refused as a stale-diff usage error naming path and line.
#[test]
fn missing_supported_target_in_root_is_refused_as_stale() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let diff = concat!(
        "diff --git a/repo/src/missing.rs b/repo/src/missing.rs\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/repo/src/missing.rs\n",
        "@@ -0,0 +1 @@\n",
        "+pub fn ghost() {}\n",
    );
    let assert = run_diff(
        repo.path(),
        &output,
        diff,
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(2);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("does not match the scanned tree"),
        "the refusal is a stale-diff usage error: {stderr}"
    );
    assert!(
        stderr.contains("src/missing.rs"),
        "the refusal names the missing path: {stderr}"
    );
    assert!(
        stderr.contains("line 1"),
        "the refusal names the first claimed new-side line: {stderr}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2 contrast: targets outside the scan
// root, with unsupported extensions, or present on disk but
// deliberately excluded from the corpus (built-in `vendor/`
// exclusion) all stay ignorable — the run passes with a zero scope.
#[test]
fn out_of_root_unsupported_and_excluded_targets_stay_ignored() -> Result<()> {
    let repo = build_repo(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", DUP_SOURCE),
        ("vendor/lib.rs", "pub fn vendored() {}\n"),
    ])?;
    let (_out_tmp, output) = output_prefix()?;
    let diff = concat!(
        "diff --git a/docs/notes.md b/docs/notes.md\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/docs/notes.md\n",
        "@@ -0,0 +1 @@\n",
        "+# outside the scan root\n",
        "diff --git a/repo/notes.md b/repo/notes.md\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/repo/notes.md\n",
        "@@ -0,0 +1 @@\n",
        "+# unsupported extension\n",
        "diff --git a/repo/vendor/lib.rs b/repo/vendor/lib.rs\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/repo/vendor/lib.rs\n",
        "@@ -0,0 +1 @@\n",
        "+pub fn vendored() {}\n",
    );
    let _assert = run_diff(
        repo.path(),
        &output,
        diff,
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(0);
    assert_zero_scope_pass(&output)?;
    Ok(())
}

/// The metadata-only 100%-similarity copy of `dup_a.rs` onto
/// `dup_b.rs` — git's record of a wholesale file duplication.
const METADATA_ONLY_COPY: &str = concat!(
    "diff --git a/repo/src/dup_a.rs b/repo/src/dup_b.rs\n",
    "similarity index 100%\n",
    "copy from repo/src/dup_a.rs\n",
    "copy to repo/src/dup_b.rs\n",
);

// [PIPELINE-DIFF-INGEST] P0-3: the metadata-only copy used to ingest
// as 0/0 added LOC — the exact duplication event this tool exists to
// catch never entered the diff scope. Every line of the copy target
// is added: it must be tagged, counted, reported as 100% duplicated,
// and must breach `--fail-over 0` with exit 3.
#[test]
fn metadata_only_copy_counts_every_line_and_breaches_the_gate() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let _assert = run_diff(
        repo.path(),
        &output,
        METADATA_ONLY_COPY,
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(3);
    let report = load_json(&output.with_extension("json"))?;
    let diff = diff_metrics(&report);
    assert_eq!(
        field(&diff, "added_loc"),
        DUP_LINES,
        "every target line is added: {report:#}"
    );
    assert_eq!(
        field(&diff, "duplicated_added_loc"),
        DUP_LINES,
        "the whole copied file is duplicated added code: {report:#}"
    );
    assert_eq!(field(&diff, "duplication_percent"), 100.0);
    assert_eq!(field(field(&diff, "threshold"), "breached"), true);
    assert_eq!(field(field(&diff, "threshold"), "source"), "cli");

    assert_eq!(clusters(&report).len(), 1, "the copy pair survives the filter");
    assert_eq!(field(&report, "clusters_outside_diff"), 0);
    let cluster = clusters(&report).first().context("the copy cluster")?;
    assert_eq!(field(cluster, "bucket"), "identical");
    assert_eq!(field(cluster, "intersects_diff"), true);
    assert_eq!(
        field(cluster, "is_newly_introduced"),
        false,
        "the copy source predates the diff: {cluster:#}"
    );
    for occurrence in occurrences(cluster) {
        let path = field(occurrence, "path").as_str().context("occurrence path")?;
        let expected_in_diff = path == "src/dup_b.rs";
        assert_eq!(
            field(occurrence, "in_diff"),
            expected_in_diff,
            "{path}: only the copy target is in the diff"
        );
    }
    let text = fs::read_to_string(output.with_extension("txt"))?;
    assert!(
        text.contains("[in diff]") && text.contains("[existing]"),
        "the copy target is badged in-diff, the source existing: {text}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3 deliberate contrast pinned by the plan:
// a pure rename with no content change moves a file — it adds no
// lines, tags nothing, and passes the same gate the copy breaches.
#[test]
fn pure_rename_adds_nothing_in_contrast_to_a_copy() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let diff = concat!(
        "diff --git a/repo/src/dup_a.rs b/repo/src/renamed.rs\n",
        "similarity index 100%\n",
        "rename from repo/src/dup_a.rs\n",
        "rename to repo/src/renamed.rs\n",
    );
    let _assert = run_diff(
        repo.path(),
        &output,
        diff,
        &["--only-changed", "--fail-over", "0"],
    )?
    .code(0);
    assert_zero_scope_pass(&output)?;
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: copy sections the tree contradicts are
// stale-diff usage errors — a missing target, a target that no longer
// byte-equals its source (the diff's `similarity index 100%` claim),
// and a missing source each exit 2 naming the offending file.
#[test]
fn copy_sections_that_disagree_with_the_tree_are_refused() -> Result<()> {
    let repo = dup_pair_repo()?;
    let (_out_tmp, output) = output_prefix()?;
    let missing_target = concat!(
        "diff --git a/repo/src/dup_a.rs b/repo/src/copied.rs\n",
        "similarity index 100%\n",
        "copy from repo/src/dup_a.rs\n",
        "copy to repo/src/copied.rs\n",
    );
    let assert = run_diff(repo.path(), &output, missing_target, &[])?.code(2);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("src/copied.rs") && stderr.contains("does not match the scanned tree"),
        "missing copy target must be a stale refusal naming the path: {stderr}"
    );

    let missing_source = concat!(
        "diff --git a/repo/src/ghost.rs b/repo/src/dup_b.rs\n",
        "similarity index 100%\n",
        "copy from repo/src/ghost.rs\n",
        "copy to repo/src/dup_b.rs\n",
    );
    let assert = run_diff(repo.path(), &output, missing_source, &[])?.code(2);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("src/ghost.rs"),
        "missing copy source must be a stale refusal naming the path: {stderr}"
    );

    let divergent = build_repo(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", &DUP_SOURCE.replace("0x2f2f", "0x3e3e")),
    ])?;
    let assert = run_diff(divergent.path(), &output, METADATA_ONLY_COPY, &[])?.code(2);
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("src/dup_b.rs") && stderr.contains("line 9"),
        "a divergent 100% copy must name the target and first divergent line: {stderr}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: below 100% similarity git also emits
// hunks (the delta against the source), but the target is still
// wholly new content — `added_loc` is the target's full line count,
// once: never just the hunk's lines, never a double count.
#[test]
fn copy_with_hunks_counts_the_whole_target_once() -> Result<()> {
    let target_source = DUP_SOURCE.replace("0x2f2f", "0x3e3e");
    let repo = build_repo(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", &target_source),
    ])?;
    let (_out_tmp, output) = output_prefix()?;
    let diff = concat!(
        "diff --git a/repo/src/dup_a.rs b/repo/src/dup_b.rs\n",
        "similarity index 90%\n",
        "copy from repo/src/dup_a.rs\n",
        "copy to repo/src/dup_b.rs\n",
        "index 1111111..2222222 100644\n",
        "--- a/repo/src/dup_a.rs\n",
        "+++ b/repo/src/dup_b.rs\n",
        "@@ -9 +9 @@\n",
        "-    acc ^ 0x2f2f\n",
        "+    acc ^ 0x3e3e\n",
    );
    let _assert = run_diff(repo.path(), &output, diff, &["--only-changed"])?.code(0);
    let report = load_json(&output.with_extension("json"))?;
    let diff_block = diff_metrics(&report);
    assert_eq!(
        field(&diff_block, "added_loc"),
        DUP_LINES,
        "the whole target counts once — not 1 hunk line, not 11: {report:#}"
    );
    let duplicated = field(&diff_block, "duplicated_added_loc")
        .as_u64()
        .context("duplicated_added_loc")?;
    let percent = field(&diff_block, "duplication_percent")
        .as_f64()
        .context("duplication_percent")?;
    let expected = 100.0 * f64::from(u32::try_from(duplicated)?)
        / f64::from(u32::try_from(DUP_LINES)?);
    assert!(
        (percent - expected).abs() < 1e-9,
        "percent {percent} must be 100*{duplicated}/{DUP_LINES}"
    );
    Ok(())
}
