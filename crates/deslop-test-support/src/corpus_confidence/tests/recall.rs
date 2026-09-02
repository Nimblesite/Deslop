//! Curated exact-copy recall asserts visibility and mass rank only.

use super::*;

/// Curated exact-copy pair.
const PAIR: [&str; 2] = ["lib/slider.dart", "lib/range_slider.dart"];

/// Manifest containing one verified exact-copy family.
fn manifest(files: &[&str], max_rank: Option<u64>) -> Value {
    json!({
        "must_find": [{
            "files": files,
            "why": "hand-verified exact copy",
            "max_rank": max_rank
        }]
    })
}

/// Runs exact-copy recall against a report.
fn judge(clusters: &[Value], max_rank: Option<u64>) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_curated_recall(&manifest(&PAIR, max_rank), &report(clusters), &mut failures);
    failures
}

#[test]
fn visible_curated_family_passes() {
    let cluster = spanning("curated", 137, 1, &PAIR);
    assert!(judge(&[cluster], None).is_empty());
}

#[test]
fn missing_curated_family_fails() {
    let cluster = spanning("elsewhere", 137, 1, &["lib/a.dart", "lib/b.dart"]);
    assert_only_failure(
        &judge(&[cluster], None),
        "recall",
        "a missing exact copy is a false negative",
        "lib/slider.dart",
        "the failure names the missed family",
    );
}

#[test]
fn hidden_curated_side_fails() {
    let cluster = hide_occurrence(spanning("curated", 137, 1, &PAIR), PAIR[1]);
    assert_only_failure(
        &judge(&[cluster], None),
        "recall",
        "recall requires both visible sides",
        "lib/slider.dart",
        "the failure names the hidden family",
    );
}

#[test]
fn rank_at_inclusive_ceiling_passes() {
    let first = spanning("other", 200, 1, &["lib/a.dart", "lib/b.dart"]);
    let curated = spanning("curated", 137, 2, &PAIR);
    assert!(judge(&[first, curated], Some(2)).is_empty());
}

#[test]
fn rank_below_ceiling_fails() {
    let first = spanning("other", 200, 1, &["lib/a.dart", "lib/b.dart"]);
    let curated = spanning("curated", 137, 2, &PAIR);
    assert_only_failure(
        &judge(&[first, curated], Some(1)),
        "recall_quality",
        "curated rank ceilings are enforceable",
        "ranks 2",
        "the failure names the rank",
    );
}

#[test]
fn absent_rank_ceiling_asserts_no_rank() {
    let first = spanning("other", 200, 1, &["lib/a.dart", "lib/b.dart"]);
    let curated = spanning("curated", 137, 2, &PAIR);
    assert!(judge(&[first, curated], None).is_empty());
}

#[test]
fn one_file_manifest_entry_fails() {
    let mut failures = Vec::new();
    check_curated_recall(
        &manifest(&[PAIR[0]], None),
        &report(&[spanning("curated", 137, 1, &PAIR)]),
        &mut failures,
    );
    assert_only_failure(
        &failures,
        "recall",
        "one file does not describe duplication",
        "[]",
        "the malformed entry cannot pass vacuously",
    );
}
