//! [CORPUS-CEILINGS] The harness's own plumbing, isolated.
//!
//! These pin the two things that decide whether a corpus test can start at
//! all: where it looks for the binary it measures, and how it reads a peak
//! RSS back out of the tool that measured it. Both are invisible to every
//! assertion in `corpus_repos.rs`, because a harness that cannot launch the
//! scan never reaches them.

use std::{
    env::consts::EXE_SUFFIX,
    path::{Path, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

use anyhow::Result;
use serde_json::{json, Value};

use super::{measurement, peak_rss_mb, release_binary_path, Measurement};

/// The binary cargo actually produces is `deslop` on Unix and `deslop.exe`
/// on Windows. Naming the bare stem makes `is_file()` false on Windows, so
/// every corpus test dies on "release binary missing" with the binary
/// sitting right there — the scan never runs and the gate reports a fault
/// that has nothing to do with the corpus.
#[test]
fn the_measured_binary_carries_the_platform_executable_suffix() {
    let path = release_binary_path();
    let expected = format!("deslop{EXE_SUFFIX}");
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some(expected.as_str()),
        "the corpus harness must look for `{expected}` — the name cargo writes on this \
         platform — not a bare stem that only resolves on Unix. Got {}.",
        path.display()
    );
}

/// The path must still be the release profile's output, not a debug build:
/// the ceilings in every manifest were measured against optimised code.
#[test]
fn the_measured_binary_comes_from_the_release_profile() {
    let path = release_binary_path();
    assert!(
        path.parent() == Some(&repo_target_release()),
        "the corpus harness must measure `target/release`, since [CORPUS-CEILINGS] budgets \
         were measured against optimised code. Got {}.",
        path.display()
    );
}

/// `target/release` under the repository root.
fn repo_target_release() -> std::path::PathBuf {
    super::repo_root().join("target").join("release")
}

/// GNU `/usr/bin/time -v` reports kbytes, and the label says so.
#[test]
fn a_gnu_peak_is_read_as_kbytes() -> Result<()> {
    let stderr = "\tMaximum resident set size (kbytes): 7340032\n";
    let parsed = peak_rss_mb(stderr)?;
    assert_eq!(parsed, 7168, "7340032 kbytes is 7168 MB");
    Ok(())
}

/// BSD `/usr/bin/time -l` reports bytes, with no unit in the label.
#[test]
fn a_bsd_peak_is_read_as_bytes() -> Result<()> {
    let stderr = "         7516192768  maximum resident set size\n";
    let parsed = peak_rss_mb(stderr)?;
    assert_eq!(parsed, 7168, "7516192768 bytes is 7168 MB");
    Ok(())
}

/// A measurement that never appeared is an error, never a zero: a silent
/// zero would clear every memory ceiling in the corpus at once.
#[test]
fn a_missing_peak_is_an_error_rather_than_zero() {
    let outcome = peak_rss_mb("nothing useful here\n");
    assert!(
        outcome.is_err(),
        "a stderr with no peak line must not yield a number, got: {outcome:?}"
    );
    let message = outcome
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        message.contains("maximum resident set size"),
        "the error must name what it could not find, got: {message}"
    );
}

/// The harness must not be fooled into reading a directory as a binary.
#[test]
fn the_measured_binary_is_not_a_directory() {
    let path = release_binary_path();
    assert!(
        !Path::new(&path).is_dir(),
        "the corpus harness resolved a directory as the binary it measures: {}",
        path.display()
    );
}

/// [CORPUS-CEILINGS] Every platform the corpus gate runs on must be able to
/// read a *true* peak. A platform with no measurement leaves every memory
/// ceiling in `corpus/*.json` unasserted while the suite still reports green.
#[test]
fn the_harness_measures_peak_rss_on_this_platform() {
    match measurement() {
        Measurement::PosixTime { flag } => assert!(
            flag == "-v" || flag == "-l",
            "`/usr/bin/time` takes `-v` (GNU) or `-l` (BSD); `{flag}` would be rejected and \
             kill every scan before a check ran"
        ),
        Measurement::WindowsPeakMonitor { script } => assert!(
            script.is_file(),
            "Windows has no `/usr/bin/time`, so the harness needs its peak monitor at {}. \
             Without it every corpus test dies before it scans anything.",
            script.display()
        ),
    }
}

/// The separator every rendered report path carries.
const WIRE_SEPARATOR: char = '/';

/// The forward-slash form every `corpus/*.json` manifest curates, and the
/// form every rendered report path must carry on every platform.
const CURATED_PAIR: [&str; 2] = ["tokio/src/io/stderr.rs", "tokio/src/io/stdout.rs"];

/// The curated files as the checks receive them.
fn curated_files() -> Vec<String> {
    CURATED_PAIR.iter().map(ToString::to_string).collect()
}

/// One visible cluster spanning the curated pair, in wire form.
fn report_spanning_pair() -> Value {
    let occurrences: Vec<Value> = CURATED_PAIR
        .iter()
        .map(|file| json!({ "path": file, "hidden": false }))
        .collect();
    json!({ "clusters": [{ "occurrences": occurrences }] })
}

/// The same report with occurrences carrying the platform separator —
/// the form the renderer emitted on Windows before gh #439, and one no
/// correct report can contain.
fn report_spanning_pair_with_native_separators() -> Value {
    let occurrences: Vec<Value> = CURATED_PAIR
        .iter()
        .map(|file| json!({ "path": file.replace(WIRE_SEPARATOR, MAIN_SEPARATOR_STR), "hidden": false }))
        .collect();
    json!({ "clusters": [{ "occurrences": occurrences }] })
}

/// The one visible cluster of `report`, or `None` when it renders none.
fn only_cluster(report: &Value) -> Option<&Value> {
    super::visible_clusters(report).into_iter().next()
}

/// True when the report's one visible cluster is shown spanning the pair.
/// A report rendering no visible cluster is false, so a vanished cluster
/// fails a positive case instead of satisfying a negative one.
fn only_cluster_shows_pair(report: &Value) -> bool {
    only_cluster(report).is_some_and(|cluster| super::cluster_shows_span(cluster, &curated_files()))
}

/// [CORPUS-RECALL] [CORPUS-PRECISION-CURATED] A report in wire form is
/// matched against the manifest exactly. This is the case the whole gate
/// rests on, and on Windows it was unreachable: the renderer emitted the
/// platform separator, so `type2_recall` reported `no cluster spans`
/// against a tokio report that held the curated whole-module rename with
/// both occurrences shown, while `precision` — the same predicate read
/// backwards — found no breach whatever the report clustered (gh #439).
#[test]
fn a_curated_pair_rendered_in_wire_form_is_spanned_and_shown() {
    let report = report_spanning_pair();
    assert!(
        super::reports_clone_spanning(&report, &curated_files()),
        "a cluster spanning the curated pair must satisfy recall; the report is {report}"
    );
    assert!(
        only_cluster_shows_pair(&report),
        "the same cluster must breach a curated precision entry naming those files; \
         the report is {report}"
    );
}

/// The predicates stay exact. A path carrying the platform separator is a
/// path the renderer must never have emitted, and it misses here rather
/// than being quietly reinterpreted — repairing it inside the harness
/// would hide the renderer defect gh #439 records from every corpus run.
/// `location_rendering::every_reported_path_is_joined_with_the_wire_separator`
/// is what holds the renderer to the wire form.
///
/// Asserts nothing where the platform separator is already the wire
/// separator, because there is no distinct native form to reject.
#[test]
fn a_native_separator_path_is_not_silently_reinterpreted() {
    if MAIN_SEPARATOR == WIRE_SEPARATOR {
        return;
    }
    let report = report_spanning_pair_with_native_separators();
    assert!(
        only_cluster(&report).is_some(),
        "the negative case needs a visible cluster, or it passes for the wrong reason: {report}"
    );
    assert!(
        !super::reports_clone_spanning(&report, &curated_files()),
        "a `{MAIN_SEPARATOR}`-separated occurrence path is not the curated path and must not \
         satisfy recall; the report is {report}"
    );
    assert!(
        !only_cluster_shows_pair(&report),
        "nor may it breach a curated precision entry; the report is {report}"
    );
}
