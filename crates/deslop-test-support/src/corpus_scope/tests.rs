//! [CORPUS-SCOPE] The scan-happened checks, in both directions.
//!
//! These are the assertions gh #342 needed and did not have. Each states
//! the failure the check exists to catch *and* the pass it must still give
//! a healthy report, so neither half can be satisfied by a check that
//! always fires or a check that never does.

use serde_json::{json, Value};

use super::check_scan_scope;
use crate::corpus::Failure;

/// A manifest that curates both bounds.
fn manifest() -> Value {
    json!({
        "expect_files_min": 100,
        "expect_clusters": { "min": 10, "max": 50 },
    })
}

/// A report analysing `files` files and rendering `clusters` clusters.
fn report(files: u64, clusters: usize) -> Value {
    json!({
        "files_analysed": files,
        "clusters": vec![json!({ "id": "c" }); clusters],
    })
}

/// The check ids a run produced, in order.
fn checks(manifest: &Value, report: &Value) -> Vec<String> {
    let mut failures: Vec<Failure> = Vec::new();
    check_scan_scope(manifest, report, &mut failures);
    failures.into_iter().map(|failure| failure.check).collect()
}

#[test]
fn a_healthy_report_inside_both_bounds_passes() {
    assert!(
        checks(&manifest(), &report(120, 30)).is_empty(),
        "120 files and 30 clusters sit inside the curated bounds; a check \
         that fires here would be a gate nobody could keep green"
    );
    assert!(
        checks(&manifest(), &report(100, 10)).is_empty(),
        "the bounds are inclusive at both ends — a repository sitting \
         exactly on its floor has not lost anything"
    );
    assert!(
        checks(&manifest(), &report(100, 50)).is_empty(),
        "and inclusive at the ceiling"
    );
}

#[test]
fn a_scan_that_analysed_nothing_is_refused() {
    assert_eq!(
        checks(&manifest(), &report(0, 0)),
        vec!["files_analysed".to_owned(), "cluster_count_band".to_owned()],
        "gh #342 shipped a scan that analysed zero files, rendered clean and \
         exited 0. Both bounds must name it — the empty report is the total \
         false negative this whole suite exists to catch"
    );
    assert_eq!(
        checks(&manifest(), &report(99, 30)),
        vec!["files_analysed".to_owned()],
        "one file short of the floor is still a scan that lost part of the \
         repository, and the cluster band must not mask it"
    );
}

#[test]
fn a_repository_wide_cluster_swing_is_refused_in_both_directions() {
    assert_eq!(
        checks(&manifest(), &report(120, 9)),
        vec!["cluster_count_band".to_owned()],
        "detection that stopped finding duplicates leaves every surviving \
         cluster correct, so only a population bound can see it"
    );
    assert_eq!(
        checks(&manifest(), &report(120, 51)),
        vec!["cluster_count_band".to_owned()],
        "and a filter that started manufacturing them is the same swing in \
         the other direction"
    );
}

#[test]
fn an_uncurated_bound_fails_rather_than_passing_vacuously() {
    assert_eq!(
        checks(&json!({}), &report(120, 30)),
        vec!["files_analysed".to_owned(), "cluster_count_band".to_owned()],
        "an absent bound is not a repository with no opinion about its own \
         size — it is a check that cannot fire, and [CORPUS-BASELINE] would \
         read that silence as evidence the defect is absent"
    );
    assert_eq!(
        checks(
            &json!({ "expect_files_min": 100, "expect_clusters": { "min": 10 } }),
            &report(120, 30)
        ),
        vec!["cluster_count_band".to_owned()],
        "a half-written band is uncurated too: without a ceiling nothing \
         refuses an explosion"
    );
}

#[test]
fn a_report_missing_the_field_is_refused_rather_than_read_as_zero() {
    assert_eq!(
        checks(&manifest(), &json!({ "clusters": [] })),
        vec!["files_analysed".to_owned(), "cluster_count_band".to_owned()],
        "a renderer that dropped `files_analysed` must fail the check, not \
         default it — a defaulted zero and a measured zero are different \
         defects and both are fatal"
    );
}
