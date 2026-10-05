//! [LIVE-SCHEDULER-REMOVAL-COST] Ignored build deletions do bounded lookup work.

use super::*;
use crate::common::store::read_single_log;

/// Empty parsed files are live even though they contribute no fingerprints.
const EMPTY_SOURCE: &str = "";
/// Enough live files to distinguish a path lookup from a corpus walk.
const QUIET_FILE_COUNT: usize = 128;
/// Upper bound on visited prefix candidates, excluding tree key comparisons.
const MAX_REMOVAL_PROBES: usize = 16;
/// The source fixture contains exactly the two halves of one clone.
const CLONE_FILES: usize = 2;
/// A missing artifact path never belonged to the analysed corpus.
const MISSING_ARTIFACT: &str = "target/build-output/removed.tmp";
/// Structured field emitted by the actual removal lookup.
const PROBE_FIELD: &str = "removal_paths_examined";
/// Make work telemetry deterministic even under a quiet parent test runner.
const LOG_FILTER_ENV: &str = "RUST_LOG";
/// The removal lookup publishes its resource count at this level.
const LOG_LEVEL: &str = "debug";
/// Rendered output base shared with the parent rerun helpers.
const REPORT_NAME: &str = "report";
/// Stable authored clone paths.
const CLONE_PATHS: [&str; CLONE_FILES] = ["Alpha.cs", "Beta.cs"];
/// The seeded report contains one ranked cluster and each replay one removal.
const SINGLE_ITEM: usize = 1;
/// Neither an unknown path nor an empty file changes a clone group.
const NO_CLONE_CHANGES: usize = 0;
/// Replay a nonexistent path without attempting to delete it again.
const TOUCH_FLAG: &str = "--rerun-touch";
/// Remove a real live file after the initial report is built.
const REMOVE_FLAG: &str = "--rerun-remove";
/// The first authored empty file, used to pin denominator updates.
const FIRST_QUIET_INDEX: usize = 0;

/// Give every empty source a stable, supported filename.
fn quiet_path(root: &Path, index: usize) -> PathBuf {
    root.join(format!("quiet-{index}.rs"))
}

/// Add real empty source files without introducing additional clone groups.
fn populate_quiet_files(root: &Path) -> Result<()> {
    for index in 0..QUIET_FILE_COUNT {
        fs::write(quiet_path(root, index), EMPTY_SOURCE)?;
    }
    Ok(())
}

/// Parse the log's numeric key/value field rather than searching source text.
fn removal_probes(log: &str) -> Result<Vec<usize>> {
    log.split_ascii_whitespace()
        .filter_map(|token| token.split_once('='))
        .filter(|(field, _)| *field == PROBE_FIELD)
        .map(|(_, value)| value.parse().map_err(Into::into))
        .collect()
}

/// Pin the full surviving clone and the live file denominator.
fn assert_surviving_clone(report: &Value, files: usize) -> Result<()> {
    assert_eq!(field(report, "files_analysed"), files);
    assert_eq!(array_len(report, "clusters"), SINGLE_ITEM);
    let clone = expect_cluster_spanning(report, &CLONE_PATHS)?;
    assert_eq!(field(clone, "kind"), NEARLY_IDENTICAL_KIND);
    assert_eq!(field(clone, "rank"), SINGLE_ITEM);
    assert_eq!(field(clone, "occurrence_count"), CLONE_FILES);
    let expected_paths: Vec<_> = CLONE_PATHS.into_iter().map(str::to_owned).collect();
    assert_eq!(occurrence_files(clone), expected_paths);
    Ok(())
}

/// Clone additions, removals and updates are all pinned independently.
fn assert_unchanged_clusters(delta: &Value) {
    assert_first_generation_span(delta);
    for key in ["clusters_added", "clusters_removed", "clusters_updated"] {
        assert_eq!(array_len(delta, key), NO_CLONE_CHANGES);
    }
}

/// Drive a real CLI replay and read the rendered report, delta and telemetry.
fn run_removal(tmp: &Path, root: &Path, flag: &str, path: &Path) -> Result<(Value, Value, String)> {
    let output = tmp.join(REPORT_NAME);
    let operations = [(flag, path.as_os_str())];
    let mut command = rerun_cmd(root, &output, SEEDED_MIN_NODES, &operations)?;
    let _assertion = command
        .env(LOG_FILTER_ENV, LOG_LEVEL)
        .args(["--log-level", LOG_LEVEL])
        .assert()
        .success();
    Ok((
        load_json(&output.with_extension("json"))?,
        load_json(&delta_path(tmp))?,
        read_single_log(tmp)?,
    ))
}

/// Check deterministic path work independently of machine speed.
fn assert_bounded_lookup(log: &str) -> Result<()> {
    let probes = removal_probes(log)?;
    assert_eq!(
        probes.len(),
        SINGLE_ITEM,
        "the removal lookup must publish its work count: {log}"
    );
    assert!(
        probes.iter().all(|count| *count <= MAX_REMOVAL_PROBES),
        "an unrelated removal must not walk every live file: {probes:?}"
    );
    Ok(())
}

#[test]
fn unknown_build_removal_preserves_report_with_bounded_path_lookup() -> Result<()> {
    let (tmp, root) = seeded_root()?;
    populate_quiet_files(&root)?;
    let missing = root.join(MISSING_ARTIFACT);
    let (report, delta, log) = run_removal(tmp.path(), &root, TOUCH_FLAG, &missing)?;
    assert_surviving_clone(&report, QUIET_FILE_COUNT.saturating_add(CLONE_FILES))?;
    assert_unchanged_clusters(&delta);
    assert_bounded_lookup(&log)
}

#[test]
fn removing_an_empty_source_updates_membership_and_preserves_the_clone() -> Result<()> {
    let (tmp, root) = seeded_root()?;
    populate_quiet_files(&root)?;
    let empty = quiet_path(&root, FIRST_QUIET_INDEX);
    let (report, delta, log) = run_removal(tmp.path(), &root, REMOVE_FLAG, &empty)?;
    let surviving_files = QUIET_FILE_COUNT
        .saturating_add(CLONE_FILES)
        .saturating_sub(SINGLE_ITEM);
    assert_surviving_clone(&report, surviving_files)?;
    assert_unchanged_clusters(&delta);
    assert_bounded_lookup(&log)
}
