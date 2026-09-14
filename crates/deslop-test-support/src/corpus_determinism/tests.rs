//! [PIPELINE-DETERMINISM] Unit coverage for the two-scan comparison.
//!
//! Every mutation below leaves the ordered cluster id vector untouched,
//! which is all the previous gate compared. Each one is a real way a
//! report can stop being reproducible, and each must be caught.

use serde_json::{json, Value};

use super::{check_reports_agree, DETERMINISM_CHECK};
use crate::corpus::Failure;

/// The cluster id both scans agree on in every case here — the field the
/// old comparison read, held constant on purpose.
const STABLE_ID: &str = "0f1e2d3c4b5a6978";

/// A second cluster id, for the reordering case.
const OTHER_ID: &str = "1122334455667788";

/// One rendered report with two clusters, as the corpus gate sees it.
fn report() -> Value {
    json!({
        "tool_version": "0.0.0-dev",
        "min_nodes": 12,
        "files_analysed": 2835,
        "clusters_hidden": 4,
        "cache_stats": { "hits": 0, "misses": 2835 },
        "metrics": { "duplication_percent": 27.822, "duplicated_loc": 91_234 },
        "clusters": [
            {
                "id": STABLE_ID,
                "bucket": "identical",
                "size": 2,
                "signals": { "fused": 1.0, "structural": 1.0, "agreement": 1.0 },
                "occurrences": [
                    { "path": "django/db/models/query.py", "start_line": 41, "end_line": 178 },
                    { "path": "django/db/models/sql/compiler.py", "start_line": 12, "end_line": 149 }
                ]
            },
            {
                "id": OTHER_ID,
                "bucket": "nearly_identical",
                "size": 2,
                "signals": { "fused": 0.91, "structural": 1.0, "agreement": 0.82 },
                "occurrences": [
                    { "path": "django/forms/fields.py", "start_line": 3, "end_line": 60 },
                    { "path": "django/forms/widgets.py", "start_line": 8, "end_line": 65 }
                ]
            }
        ]
    })
}

/// The failures `check_reports_agree` records for one pair of reports.
fn compare(first: &Value, second: &Value) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_reports_agree(first, second, &mut failures);
    failures
}

/// The single failure detail a comparison produced, or the empty string
/// when it produced none.
fn detail(failures: &[Failure]) -> String {
    failures
        .first()
        .map(|failure| failure.detail.clone())
        .unwrap_or_default()
}

/// Asserts the pair is rejected under the determinism check, and that
/// the message names `expected` so a real corpus failure points at the
/// field that moved rather than at two whole-repository reports.
fn assert_rejected(mutated: &Value, expected: &str) {
    let failures = compare(&report(), mutated);
    assert_eq!(
        failures.len(),
        1,
        "exactly one determinism failure was expected for {expected}: {failures:?}"
    );
    assert_eq!(
        failures.first().map(|failure| failure.check.as_str()),
        Some(DETERMINISM_CHECK)
    );
    let detail = detail(&failures);
    assert!(
        detail.contains(expected),
        "the failure must name {expected} so the moving field is diagnosable: {detail}"
    );
}

/// Replaces the value at `pointer`, asserting the pointer resolved so a
/// typo cannot turn a mutation case into a no-op that quietly passes.
fn mutate(pointer: &str, value: Value) -> Value {
    let mut mutated = report();
    let applied = mutated
        .pointer_mut(pointer)
        .map(|slot| *slot = value)
        .is_some();
    assert!(applied, "{pointer} is not a field of the fixture report");
    mutated
}

/// The fixture report with its cluster array rewritten by `edit`.
fn with_clusters(edit: impl FnOnce(&mut Vec<Value>)) -> Value {
    let mut edited = report();
    let found = edited
        .get_mut("clusters")
        .and_then(Value::as_array_mut)
        .map(edit)
        .is_some();
    assert!(found, "the fixture report carries a clusters array");
    edited
}

#[test]
fn two_scans_of_one_corpus_state_agree() {
    assert!(
        compare(&report(), &report()).is_empty(),
        "a report compared with itself is reproducible; a check that fires \
         here is a gate nobody could keep green"
    );
}

#[test]
fn a_warm_cache_on_the_second_run_is_not_a_disagreement() {
    let second = mutate("/cache_stats", json!({ "hits": 2835, "misses": 0 }));
    assert!(
        compare(&report(), &second).is_empty(),
        "cache_stats describes the run, not the corpus — the second scan reads \
         the first scan's fingerprint cache by design"
    );
}

#[test]
fn a_moved_occurrence_range_fails_determinism_with_the_ids_unchanged() {
    assert_rejected(
        &mutate("/clusters/0/occurrences/0/end_line", json!(177)),
        "clusters[0].occurrences[0].end_line",
    );
}

#[test]
fn a_moved_occurrence_path_fails_determinism_with_the_ids_unchanged() {
    assert_rejected(
        &mutate(
            "/clusters/0/occurrences/1/path",
            json!("django/db/models/sql/where.py"),
        ),
        "clusters[0].occurrences[1].path",
    );
}

#[test]
fn a_changed_bucket_fails_determinism_with_the_ids_unchanged() {
    assert_rejected(
        &mutate("/clusters/0/bucket", json!("nearly_identical")),
        "clusters[0].bucket",
    );
}

#[test]
fn a_changed_signal_fails_determinism_with_the_ids_unchanged() {
    assert_rejected(
        &mutate("/clusters/1/signals/agreement", json!(0.79)),
        "clusters[1].signals.agreement",
    );
}

#[test]
fn a_changed_duplication_percentage_fails_determinism() {
    assert_rejected(
        &mutate("/metrics/duplication_percent", json!(27.821)),
        "metrics.duplication_percent",
    );
}

#[test]
fn a_changed_hidden_count_fails_determinism() {
    assert_rejected(&mutate("/clusters_hidden", json!(5)), "clusters_hidden");
}

#[test]
fn a_changed_files_analysed_fails_determinism() {
    assert_rejected(&mutate("/files_analysed", json!(2834)), "files_analysed");
}

#[test]
fn reordering_the_same_clusters_fails_determinism() {
    let reordered = with_clusters(|clusters| clusters.reverse());
    assert_rejected(&reordered, "clusters[0]");
}

#[test]
fn a_dropped_cluster_fails_determinism() {
    let shorter = with_clusters(|clusters| {
        let _removed = clusters.pop();
    });
    assert_rejected(&shorter, "clusters: 2 entries vs 1");
}

#[test]
fn a_cluster_without_a_string_id_is_malformed_not_skipped() {
    let malformed = mutate("/clusters/1/id", json!(null));
    let failures = compare(&report(), &malformed);
    assert_eq!(
        failures.len(),
        1,
        "malformed input is a failure, not something to skip: {failures:?}"
    );
    let detail = detail(&failures);
    assert!(
        detail.contains("rank 1") && detail.contains("no string `id`"),
        "silently dropping the cluster is what let two disagreeing reports \
         compare equal: {detail}"
    );
}

#[test]
fn a_gained_report_field_fails_determinism() {
    let mut gained = report();
    let inserted = gained.as_object_mut().map(|object| {
        let _previous = object.insert("clusters_outside_diff".to_owned(), json!(3));
    });
    assert!(inserted.is_some(), "the fixture report is a JSON object");
    assert_rejected(&gained, "clusters_outside_diff");
}
