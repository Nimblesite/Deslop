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

use std::{fs, path::PathBuf};

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

/// The strictest gate there is, under diff scope: any duplicated added
/// line fails the run.
const ZERO_GATE: &[&str] = &["--only-changed", "--fail-over", "0"];

/// One scenario's live tempdirs and the report prefix inside them. The
/// working directory is the corpus tempdir and the scan root its
/// `repo/` subdirectory; the reports land in a second tempdir so no
/// rendered report is ever discovered by the scan.
struct Scenario {
    /// Working directory: holds the `repo/` scan root.
    root: tempfile::TempDir,
    /// Kept alive so [`Self::output`] stays on disk for the whole test.
    _reports: tempfile::TempDir,
    /// Report path prefix; `.json` / `.txt` / `.html` are appended.
    output: PathBuf,
}

impl Scenario {
    /// A scenario over `files`, given as `repo/`-relative path/content
    /// pairs written under `<root>/repo`.
    fn with_files(files: &[(&str, &str)]) -> Result<Self> {
        let root = tempfile::tempdir()?;
        for (path, content) in files {
            let absolute = root.path().join("repo").join(path);
            let parent = absolute.parent().context("file path has a parent")?;
            fs::create_dir_all(parent)?;
            fs::write(absolute, content)?;
        }
        let reports = tempfile::tempdir()?;
        let output = reports.path().join("report");
        Ok(Self {
            root,
            _reports: reports,
            output,
        })
    }

    /// The byte-identical `dup_a.rs` / `dup_b.rs` pair every scenario
    /// starts from.
    fn dup_pair() -> Result<Self> {
        Self::with_files(&[("src/dup_a.rs", DUP_SOURCE), ("src/dup_b.rs", DUP_SOURCE)])
    }

    /// Runs `deslop repo --output <prefix> --no-incremental --embeddings
    /// off --diff - <extra...>` from the scenario root, feeding `diff`
    /// on stdin.
    fn run(&self, diff: &str, extra: &[&str]) -> Result<Assert> {
        let mut cmd = Command::cargo_bin("deslop")?;
        let _configured = cmd
            .current_dir(self.root.path())
            .arg("repo")
            .arg("--output")
            .arg(&self.output)
            .arg("--no-incremental")
            .args(["--embeddings", "off", "--diff", "-"])
            .args(extra)
            .write_stdin(diff);
        Ok(cmd.assert())
    }

    /// Runs a diff that must be refused and returns the stderr the
    /// refusal wrote, having asserted the usage-error exit code.
    fn refusal_stderr(&self, diff: &str, extra: &[&str]) -> Result<String> {
        let assert = self.run(diff, extra)?.code(2);
        Ok(String::from_utf8_lossy(&assert.get_output().stderr).into_owned())
    }

    /// The JSON report the last run wrote.
    fn report(&self) -> Result<Value> {
        load_json(&self.output.with_extension("json"))
    }

    /// Runs one "stays ignorable" scenario end to end and asserts the
    /// whole shape they share: the diff clears a zero ceiling under
    /// `--only-changed` (exit 0) with no added lines, no surviving
    /// cluster, and the untouched legacy cluster omitted-but-counted.
    fn assert_ignorable(&self, diff: &str) -> Result<()> {
        let _assert = self.run(diff, ZERO_GATE)?.code(0);
        let report = self.report()?;
        let scope = diff_metrics(&report);
        assert_eq!(
            field(&scope, "added_loc"),
            0,
            "no in-scope lines: {report:#}"
        );
        assert_eq!(field(&scope, "duplicated_added_loc"), 0);
        assert_eq!(field(&scope, "duplication_percent"), 0.0);
        assert_eq!(field(field(&scope, "threshold"), "breached"), false);
        assert_eq!(clusters(&report).len(), 0, "nothing intersects the diff");
        assert_eq!(
            field(&report, "clusters_outside_diff"),
            1,
            "the untouched dup pair is omitted, not lost: {report:#}"
        );
        Ok(())
    }
}

/// The `metrics.diff` block of a report.
fn diff_metrics(report: &Value) -> Value {
    field(field(report, "metrics"), "diff").clone()
}

// [PIPELINE-DIFF-INGEST] P0-1: `diff nonsense` + a hunk used to parse
// as a pathless section the verifier ignored — exit 0, zero added
// LOC, every legacy cluster omitted under `--only-changed`. A hunk
// header in a section that never saw `+++` must be a parse refusal
// (usage error, exit 2) naming the offending diff line.
#[test]
fn hunk_without_a_target_line_is_refused_naming_the_line() -> Result<()> {
    let scenario = Scenario::dup_pair()?;
    let stderr = scenario.refusal_stderr("diff nonsense\n@@ -0,0 +1 @@\n+x\n", ZERO_GATE)?;
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
        !scenario.output.with_extension("json").exists(),
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
    let scenario = Scenario::dup_pair()?;
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
    scenario.assert_ignorable(diff)?;
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2: a valid new-file hunk for a supported
// file inside the scan root that exists neither in the corpus nor on
// disk used to be silently ignored — exit 0 with zero scope. It must
// be refused as a stale-diff usage error naming path and line.
#[test]
fn missing_supported_target_in_root_is_refused_as_stale() -> Result<()> {
    let scenario = Scenario::dup_pair()?;
    let diff = concat!(
        "diff --git a/repo/src/missing.rs b/repo/src/missing.rs\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/repo/src/missing.rs\n",
        "@@ -0,0 +1 @@\n",
        "+pub fn ghost() {}\n",
    );
    let stderr = scenario.refusal_stderr(diff, ZERO_GATE)?;
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
    let scenario = Scenario::with_files(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", DUP_SOURCE),
        ("vendor/lib.rs", "pub fn vendored() {}\n"),
    ])?;
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
    scenario.assert_ignorable(diff)?;
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
    let scenario = Scenario::dup_pair()?;
    let _assert = scenario.run(METADATA_ONLY_COPY, ZERO_GATE)?.code(3);
    let report = scenario.report()?;
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

    assert_eq!(
        clusters(&report).len(),
        1,
        "the copy pair survives the filter"
    );
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
        let path = field(occurrence, "path")
            .as_str()
            .context("occurrence path")?;
        let expected_in_diff = path == "src/dup_b.rs";
        assert_eq!(
            field(occurrence, "in_diff"),
            expected_in_diff,
            "{path}: only the copy target is in the diff"
        );
    }
    let text = fs::read_to_string(scenario.output.with_extension("txt"))?;
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
    let scenario = Scenario::dup_pair()?;
    let diff = concat!(
        "diff --git a/repo/src/dup_a.rs b/repo/src/renamed.rs\n",
        "similarity index 100%\n",
        "rename from repo/src/dup_a.rs\n",
        "rename to repo/src/renamed.rs\n",
    );
    scenario.assert_ignorable(diff)?;
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: copy sections the tree contradicts are
// stale-diff usage errors — a missing target, a target that no longer
// byte-equals its source (the diff's `similarity index 100%` claim),
// and a missing source each exit 2 naming the offending file.
#[test]
fn copy_sections_that_disagree_with_the_tree_are_refused() -> Result<()> {
    let scenario = Scenario::dup_pair()?;
    let missing_target = concat!(
        "diff --git a/repo/src/dup_a.rs b/repo/src/copied.rs\n",
        "similarity index 100%\n",
        "copy from repo/src/dup_a.rs\n",
        "copy to repo/src/copied.rs\n",
    );
    let stderr = scenario.refusal_stderr(missing_target, &[])?;
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
    let stderr = scenario.refusal_stderr(missing_source, &[])?;
    assert!(
        stderr.contains("src/ghost.rs"),
        "missing copy source must be a stale refusal naming the path: {stderr}"
    );

    let divergent = Scenario::with_files(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", &DUP_SOURCE.replace("0x2f2f", "0x3e3e")),
    ])?;
    let stderr = divergent.refusal_stderr(METADATA_ONLY_COPY, &[])?;
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
    let scenario = Scenario::with_files(&[
        ("src/dup_a.rs", DUP_SOURCE),
        ("src/dup_b.rs", &target_source),
    ])?;
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
    let _assert = scenario.run(diff, &["--only-changed"])?.code(0);
    let report = scenario.report()?;
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
    let expected =
        100.0 * f64::from(u32::try_from(duplicated)?) / f64::from(u32::try_from(DUP_LINES)?);
    assert!(
        (percent - expected).abs() < 1e-9,
        "percent {percent} must be 100*{duplicated}/{DUP_LINES}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] An **empty** `+++ ` payload used to parse as a
// valid target: `new_side_path` returned `Some("")`, the section was
// marked as having seen its target, and the verifier then discarded the
// empty path as out-of-root — exit 0, zero added LOC, every repository
// cluster omitted under `--only-changed`. A truncated target header
// therefore erased the entire changed-code population and let a breached
// repository pass the changed-code gate: a false negative at the merge
// gate, produced by malformed input the fail-closed contract says is a
// usage error. It must be refused (exit 2) naming the offending line.
#[test]
fn empty_new_side_target_is_refused_naming_the_line() -> Result<()> {
    let scenario = Scenario::dup_pair()?;
    let stderr = scenario.refusal_stderr(
        concat!(
            "diff --git a/repo/src/ghost.rs b/repo/src/ghost.rs\n",
            "new file mode 100644\n",
            "--- /dev/null\n",
            "+++ \n",
            "@@ -0,0 +1,1 @@\n",
            "+pub fn duplicated() {}\n",
        ),
        ZERO_GATE,
    )?;
    assert!(
        stderr.contains("invalid unified diff"),
        "the refusal is a parse error: {stderr}"
    );
    assert!(
        stderr.contains("line 4"),
        "the refusal names the offending `+++` line: {stderr}"
    );
    assert!(
        stderr.contains("names no path"),
        "the refusal says the target names nothing: {stderr}"
    );
    assert!(
        !scenario.output.with_extension("json").exists(),
        "a refused diff must not produce a report"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] The positive control for the refusal above:
// `+++ /dev/null` is a *seen* target meaning "deleted", not an absent
// one, and must keep ingesting as an empty scope. Without this pin the
// obvious over-correction — treating every falsy target as missing —
// would turn every deletion section into a usage error.
#[test]
fn dev_null_target_is_not_an_empty_target() -> Result<()> {
    let scenario = Scenario::dup_pair()?;
    scenario.assert_ignorable(concat!(
        "diff --git a/repo/src/gone.rs b/repo/src/gone.rs\n",
        "deleted file mode 100644\n",
        "--- a/repo/src/gone.rs\n",
        "+++ /dev/null\n",
        "@@ -1 +0,0 @@\n",
        "-pub fn gone() {}\n",
    ))?;
    Ok(())
}
