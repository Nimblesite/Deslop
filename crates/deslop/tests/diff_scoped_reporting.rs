//! Diff-scoped reporting end-to-end ([CLI-ARG-DIFF],
//! [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE],
//! [CLI-ARG-ONLY-CHANGED]).
//!
//! Drives the CLI against `tests/fixtures/diff-scope`, which models the
//! CI flow: the working directory is the repository root, the scan root
//! is the `repo/` subdirectory, and the committed patches carry
//! git-style `a/` / `b/` prefixes with repo-root-relative paths. The
//! diff introduces one wholly-new clone pair (`fresh_a` / `fresh_b`),
//! adds one copy of an untouched helper (`caller.rs` cloning
//! `helper.rs`), leaves one legacy pair untouched (`legacy_a` /
//! `legacy_b`), and names one file outside the scan root
//! (`docs/notes.md`) that ingest must ignore.

mod common;

use std::{fs, path::PathBuf};

use anyhow::Context as _;
use assert_cmd::Command;
use serde_json::Value;

use crate::common::{clusters, field, fixture, load_json, occurrences, Result};

/// New-side added-line spans the committed `change.patch` carries for
/// files inside the scan root, keyed by scan-root-relative path.
/// `docs/notes.md` is deliberately absent — it sits outside the root.
const ADDED_SPANS: &[(&str, u64, u64)] = &[
    ("src/caller.rs", 8, 21),
    ("src/fresh_a.rs", 1, 12),
    ("src/fresh_b.rs", 1, 12),
];

/// Total added lines inside the scan root: 14 in `caller.rs` plus 12 in
/// each fresh file. The 2 added lines of `docs/notes.md` must never be
/// counted ([METRICS-DIFF-SCOPE]).
const ADDED_LOC: u64 = 38;

/// Builds `deslop repo --output <out> --no-incremental <extra...>` with
/// the fixture root as the working directory, so diff paths resolve the
/// way they do in CI ([CLI-ARG-DIFF]).
fn diff_cmd(output_prefix: &std::path::Path, extra: &[&str]) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _ = cmd
        .current_dir(fixture("diff-scope"))
        .arg("repo")
        .arg("--output")
        .arg(output_prefix)
        .arg("--no-incremental")
        .args(extra);
    Ok(cmd)
}

/// Runs the CLI with `extra` flags, asserts success, and returns the
/// parsed JSON report plus the output prefix (for `.txt` / `.html`).
fn run_ok(extra: &[&str]) -> Result<(Value, PathBuf, tempfile::TempDir)> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assert = diff_cmd(&output, extra)?.assert().success();
    let report = load_json(&output.with_extension("json"))?;
    Ok((report, output, tmp))
}

/// Finds the cluster whose occurrence paths exactly match `paths`
/// (order-insensitive), reporting the full cluster list otherwise.
fn cluster_with_paths<'a>(report: &'a Value, paths: &[&str]) -> Result<&'a Value> {
    let want: std::collections::BTreeSet<&str> = paths.iter().copied().collect();
    clusters(report)
        .iter()
        .find(|cluster| {
            let got: std::collections::BTreeSet<&str> = occurrences(cluster)
                .iter()
                .filter_map(|occ| occ.get("path").and_then(Value::as_str))
                .collect();
            got == want
        })
        .with_context(|| format!("no cluster with paths {paths:?} in {report:#}"))
}

/// The occurrence in `cluster` whose path is `path`.
fn occurrence_at<'a>(cluster: &'a Value, path: &str) -> Result<&'a Value> {
    occurrences(cluster)
        .iter()
        .find(|occ| occ.get("path").and_then(Value::as_str) == Some(path))
        .with_context(|| format!("no occurrence at {path} in {cluster:#}"))
}

/// `metrics` with the `diff` block removed, for mechanical-field
/// byte-identity comparisons across `--diff` on/off runs.
fn mechanical_metrics(report: &Value) -> Value {
    let mut metrics = field(report, "metrics").clone();
    if let Some(map) = metrics.as_object_mut() {
        let _ = map.remove("diff");
    }
    metrics
}

/// Sorted cluster-id set of a report.
fn id_set(report: &Value) -> std::collections::BTreeSet<String> {
    clusters(report)
        .iter()
        .filter_map(|cluster| cluster.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// True when 1-indexed `line` falls in any committed added span for
/// `path`.
fn in_added_span(path: &str, line: u64) -> bool {
    ADDED_SPANS
        .iter()
        .any(|(span_path, start, end)| *span_path == path && (*start..=*end).contains(&line))
}

/// Re-derives `duplicated_added_loc` from the report itself: the union,
/// per file, of non-hidden occurrence line ranges intersected with the
/// added spans, over clusters with >= 2 non-hidden occurrences — the
/// same projection as `duplicated_loc` ([METRICS-DIFF-SCOPE]).
fn rederive_duplicated_added(report: &Value) -> Result<u64> {
    let mut per_file: std::collections::BTreeMap<&str, std::collections::BTreeSet<u64>> =
        std::collections::BTreeMap::new();
    for cluster in clusters(report) {
        let visible: Vec<&Value> = occurrences(cluster)
            .iter()
            .filter(|occ| occ.get("hidden").and_then(Value::as_bool) == Some(false))
            .collect();
        if visible.len() < 2 {
            continue;
        }
        for occ in visible {
            let path = occ
                .get("path")
                .and_then(Value::as_str)
                .context("occurrence path")?;
            let start = occ
                .get("start_line")
                .and_then(Value::as_u64)
                .context("start_line")?;
            let end = occ
                .get("end_line")
                .and_then(Value::as_u64)
                .context("end_line")?;
            for line in start..=end {
                if in_added_span(path, line) {
                    let _ = per_file.entry(path).or_default().insert(line);
                }
            }
        }
    }
    per_file
        .values()
        .map(|lines| u64::try_from(lines.len()).context("line count fits u64"))
        .sum()
}

// [OUTPUT-SCHEMA-DIFF-TAGS] Without --diff, no diff field may appear —
// per-run optionality means field absence, not `null` noise.
#[test]
fn no_diff_run_omits_every_diff_field() -> Result<()> {
    let (report, output, _tmp) = run_ok(&[])?;
    assert_eq!(clusters(&report).len(), 3, "fixture yields three clusters");
    let raw = fs::read_to_string(output.with_extension("json"))?;
    for key in [
        "in_diff",
        "intersects_diff",
        "is_newly_introduced",
        "clusters_outside_diff",
        "duplicated_added_loc",
    ] {
        assert!(
            !raw.contains(key),
            "report without --diff must not carry {key}: {raw}"
        );
    }
    assert!(
        field(&report, "metrics").get("diff").is_none(),
        "metrics.diff must be absent without --diff"
    );
    Ok(())
}

// [OUTPUT-SCHEMA-DIFF-TAGS] + [METRICS-DIFF-SCOPE]: the four
// populations tag correctly and the added-line metrics are transparent
// recomputations of the report's own occurrence evidence.
#[test]
fn diff_tags_the_four_populations_and_metrics_add_up() -> Result<()> {
    let (baseline, _base_out, _tmp_a) = run_ok(&[])?;
    let (report, _output, _tmp_b) = run_ok(&["--diff", "patches/change.patch"])?;
    assert_eq!(clusters(&report).len(), 3, "tagging never drops clusters");

    let fresh = cluster_with_paths(&report, &["src/fresh_a.rs", "src/fresh_b.rs"])?;
    assert_eq!(field(fresh, "intersects_diff"), true);
    assert_eq!(field(fresh, "is_newly_introduced"), true);
    assert_eq!(field(fresh, "bucket"), "identical");
    for path in ["src/fresh_a.rs", "src/fresh_b.rs"] {
        assert_eq!(
            field(occurrence_at(fresh, path)?, "in_diff"),
            true,
            "{path} is wholly added"
        );
    }

    let mixed = cluster_with_paths(&report, &["src/helper.rs", "src/caller.rs"])?;
    assert_eq!(field(mixed, "intersects_diff"), true);
    assert_eq!(
        field(mixed, "is_newly_introduced"),
        false,
        "one occurrence predates the diff, so the cluster is not newly introduced"
    );
    assert_eq!(
        field(occurrence_at(mixed, "src/caller.rs")?, "in_diff"),
        true
    );
    assert_eq!(
        field(occurrence_at(mixed, "src/helper.rs")?, "in_diff"),
        false
    );

    let legacy = cluster_with_paths(&report, &["src/legacy_a.rs", "src/legacy_b.rs"])?;
    assert_eq!(field(legacy, "intersects_diff"), false);
    assert_eq!(field(legacy, "is_newly_introduced"), false);
    assert_eq!(field(legacy, "bucket"), "identical");
    for path in ["src/legacy_a.rs", "src/legacy_b.rs"] {
        assert_eq!(field(occurrence_at(legacy, path)?, "in_diff"), false);
    }

    let diff_metrics = field(field(&report, "metrics"), "diff");
    assert_eq!(
        field(diff_metrics, "added_loc"),
        ADDED_LOC,
        "38 added lines inside the scan root; docs/notes.md's 2 lines excluded"
    );
    let duplicated_added = field(diff_metrics, "duplicated_added_loc")
        .as_u64()
        .context("duplicated_added_loc")?;
    assert_eq!(
        duplicated_added,
        rederive_duplicated_added(&report)?,
        "duplicated_added_loc must equal the occurrence-evidence recomputation"
    );
    assert!(
        duplicated_added >= 24,
        "both fresh files are entirely duplicated added code, got {duplicated_added}"
    );
    let percent = field(diff_metrics, "duplication_percent")
        .as_f64()
        .context("duplication_percent")?;
    let expected =
        100.0 * f64::from(u32::try_from(duplicated_added)?) / f64::from(u32::try_from(ADDED_LOC)?);
    assert!(
        (percent - expected).abs() < 1e-9,
        "diff percent {percent} must be 100*{duplicated_added}/{ADDED_LOC}"
    );
    assert_eq!(
        field(field(diff_metrics, "threshold"), "source"),
        "none",
        "no gate rerouting without --only-changed"
    );

    assert_eq!(
        mechanical_metrics(&baseline),
        mechanical_metrics(&report),
        "--diff must never change the mechanical metrics"
    );
    assert_eq!(
        id_set(&baseline),
        id_set(&report),
        "--diff must never change cluster identity"
    );
    Ok(())
}

// [CLI-ARG-ONLY-CHANGED]: the legacy cluster is filtered, counted, and
// the renderers speak the delta language.
#[test]
fn only_changed_filters_untouched_clusters_and_renders_the_delta() -> Result<()> {
    let (full, _full_out, _tmp_a) = run_ok(&["--diff", "patches/change.patch"])?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let assert = diff_cmd(
        &output,
        &["--diff", "patches/change.patch", "--only-changed"],
    )?
    .assert()
    .success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    let report = load_json(&output.with_extension("json"))?;

    assert_eq!(
        clusters(&report).len(),
        2,
        "the untouched legacy cluster is omitted"
    );
    assert_eq!(field(&report, "clusters_outside_diff"), 1);
    for cluster in clusters(&report) {
        assert_eq!(
            field(cluster, "intersects_diff"),
            true,
            "every surviving cluster intersects the diff: {cluster:#}"
        );
    }
    let all_paths: Vec<&str> = clusters(&report)
        .iter()
        .flat_map(occurrences)
        .filter_map(|occ| occ.get("path").and_then(Value::as_str))
        .collect();
    assert!(
        !all_paths.iter().any(|path| path.contains("legacy")),
        "no legacy path may survive --only-changed: {all_paths:?}"
    );
    // [METRICS-REPO] through [METRICS-DIFF-SCOPE]: the banner count
    // follows the filtered body, the repo-wide count stays recoverable
    // as clusters_total + clusters_outside_diff, and every line metric
    // is untouched by filtering.
    let mut full_metrics = mechanical_metrics(&full);
    let mut filtered_metrics = mechanical_metrics(&report);
    assert_eq!(field(&filtered_metrics, "clusters_total"), 2);
    assert_eq!(field(&full_metrics, "clusters_total"), 3);
    for metrics in [&mut full_metrics, &mut filtered_metrics] {
        if let Some(map) = metrics.as_object_mut() {
            let _ = map.remove("clusters_total");
        }
    }
    assert_eq!(
        full_metrics, filtered_metrics,
        "--only-changed filters clusters, never the line metrics"
    );
    let surviving = id_set(&report);
    assert!(
        surviving.is_subset(&id_set(&full)),
        "filtered ids must be a subset of the full run's ids"
    );
    assert!(
        stderr
            .contains("1 group(s) newly introduced by this diff, 1 cross-file with untouched code"),
        "stderr summary must lead with the four-figure delta: {stderr}"
    );

    let text = fs::read_to_string(output.with_extension("txt"))?;
    assert!(
        text.contains(
            "delta: 2 cluster(s) intersect the diff — 1 newly introduced, \
             1 cross-file with untouched code; 1 untouched cluster(s) omitted"
        ),
        "text report must carry the four-figure delta summary: {text}"
    );
    assert!(
        text.contains("[in diff]") && text.contains("[existing]"),
        "text report must badge occurrences: {text}"
    );
    let html = fs::read_to_string(output.with_extension("html"))?;
    assert!(
        html.contains("in-diff"),
        "html cards must carry the in-diff class: missing from html output"
    );
    assert!(
        html.contains("[in diff]") && html.contains("[existing]"),
        "html must badge occurrences through the shared renderer"
    );
    Ok(())
}

// [METRICS-DIFF-SCOPE] + [EXIT-CODES]: under --only-changed the gate
// reads the diff-scoped percentage, so legacy debt cannot fail a
// pre-merge check — and new duplication still does.
#[test]
fn only_changed_gate_reads_the_diff_percentage() -> Result<()> {
    // Clean diff over a legacy-heavy repo: repo gate breached, run passes.
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let assert = diff_cmd(
        &output,
        &[
            "--diff",
            "patches/empty.patch",
            "--only-changed",
            "--fail-over",
            "0",
        ],
    )?
    .assert()
    .code(0);
    let clean_stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    let report = load_json(&output.with_extension("json"))?;
    let metrics = field(&report, "metrics");
    assert_eq!(
        field(field(metrics, "threshold"), "breached"),
        true,
        "the repo-wide verdict stays honest even when the diff gate passes"
    );
    let diff_metrics = field(metrics, "diff");
    assert_eq!(field(diff_metrics, "added_loc"), 0);
    assert_eq!(field(diff_metrics, "duplicated_added_loc"), 0);
    assert_eq!(field(diff_metrics, "duplication_percent"), 0.0);
    assert_eq!(field(field(diff_metrics, "threshold"), "breached"), false);
    assert_eq!(field(field(diff_metrics, "threshold"), "source"), "cli");
    assert_eq!(clusters(&report).len(), 0, "an empty diff touches nothing");
    assert_eq!(field(&report, "clusters_outside_diff"), 3);
    assert_eq!(
        field(metrics, "clusters_total"),
        0,
        "the banner counts the filtered body ([METRICS-REPO])"
    );
    // [METRICS-DIFF-SCOPE]: a filtered-empty run must not claim the
    // codebase is clean while naming the legacy debt it omitted.
    assert!(
        clean_stderr.contains("no diff-affected duplication — 3 untouched group(s) omitted"),
        "the clean-diff summary names the diff scope, not the codebase: {clean_stderr}"
    );
    assert!(
        !clean_stderr.contains("your codebase is clean"),
        "a filtered-empty run must not claim the codebase is clean: {clean_stderr}"
    );
    // The governing clean diff gate renders ok even though the repo
    // gate is breached — page and exit code agree.
    let clean_html = fs::read_to_string(output.with_extension("html"))?;
    assert!(
        clean_html.contains("metrics-banner metrics-banner--ok"),
        "the HTML banner follows the governing (clean) diff gate"
    );
    assert!(
        clean_html.contains("diff threshold 0.00% (ok)"),
        "the HTML banner names the governing diff verdict"
    );

    // A diff that introduces duplication trips the same gate.
    let tmp2 = tempfile::tempdir()?;
    let output2 = tmp2.path().join("report");
    let _assert = diff_cmd(
        &output2,
        &[
            "--diff",
            "patches/change.patch",
            "--only-changed",
            "--fail-over",
            "0",
        ],
    )?
    .assert()
    .code(3);
    let breached = load_json(&output2.with_extension("json"))?;
    let verdict = field(field(field(&breached, "metrics"), "diff"), "threshold");
    assert_eq!(field(verdict, "breached"), true);
    assert_eq!(field(verdict, "source"), "cli");
    assert_eq!(field(verdict, "percent"), 0.0);
    // The page must agree with exit 3: the governing diff gate breached.
    let breached_html = fs::read_to_string(output2.with_extension("html"))?;
    assert!(
        breached_html.contains("metrics-banner metrics-banner--breached"),
        "the HTML banner follows the governing (breached) diff gate"
    );
    assert!(
        breached_html.contains("diff threshold 0.00% (breached)"),
        "the HTML banner names the governing diff verdict"
    );

    // Without --only-changed the repo-wide gate governs, diff or not.
    let tmp3 = tempfile::tempdir()?;
    let output3 = tmp3.path().join("report");
    let _assert = diff_cmd(
        &output3,
        &["--diff", "patches/empty.patch", "--fail-over", "0"],
    )?
    .assert()
    .code(3);
    Ok(())
}

// [CLI-ARG-DIFF] verification: a diff that does not byte-match the
// scanned tree is refused as a usage error naming file and line.
#[test]
fn stale_diff_is_refused_with_file_and_line() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let assert = diff_cmd(&output, &["--diff", "patches/stale.patch"])?
        .assert()
        .code(2);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("caller.rs"),
        "refusal must name the mismatching file: {stderr}"
    );
    assert!(
        stderr.contains('6'),
        "refusal must name the mismatching new-side line: {stderr}"
    );
    Ok(())
}

// [CLI-ARG-DIFF] grammar: malformed diff text and a missing diff file
// are usage errors (exit 2), never analysis failures.
#[test]
fn malformed_or_missing_diff_is_a_usage_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let assert = diff_cmd(&output, &["--diff", "patches/malformed.patch"])?
        .assert()
        .code(2);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("diff"),
        "malformed refusal must say what failed: {stderr}"
    );
    let tmp2 = tempfile::tempdir()?;
    let output2 = tmp2.path().join("report");
    let _assert = diff_cmd(&output2, &["--diff", "patches/does-not-exist.patch"])?
        .assert()
        .code(2);
    Ok(())
}

// [CLI-ARG-DIFF] `--diff -` reads the unified diff from stdin.
#[test]
fn diff_from_stdin_tags_identically() -> Result<()> {
    let patch = fs::read_to_string(fixture("diff-scope").join("patches/change.patch"))?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = diff_cmd(&output, &["--diff", "-"])?;
    let _assert = cmd.write_stdin(patch).assert().success();
    let report = load_json(&output.with_extension("json"))?;
    let fresh = cluster_with_paths(&report, &["src/fresh_a.rs", "src/fresh_b.rs"])?;
    assert_eq!(field(fresh, "is_newly_introduced"), true);
    assert_eq!(
        field(field(field(&report, "metrics"), "diff"), "added_loc"),
        ADDED_LOC
    );
    Ok(())
}
