//! [CORPUS-RECALL] curated `recall` / `recall_quality` — every
//! hand-verified byte-identical duplicate in a manifest's `must_find` list
//! must be reported, shown, labelled `identical`, and ranked where a user
//! finds it.
//!
//! The check this replaces asked only whether *some* cluster's occurrence
//! paths covered the curated files. Each case below is a report that
//! satisfied that and should not have.

use super::*;

/// The pair every case below curates.
const PAIR: [&str; 2] = ["lib/slider_parts.dart", "lib/range_slider_parts.dart"];

/// A manifest curating one hand-verified byte-identical clone, with an
/// optional rank ceiling.
fn manifest_with_clone(files: &[&str], max_rank: Option<u64>) -> Value {
    let ranked = match max_rank {
        Some(rank) => json!(rank),
        None => Value::Null,
    };
    json!({
        "must_find": [{
            "files": files,
            "why": "137 byte-identical lines, hand-verified by an empty diff",
            "verified": "diff of the two ranges is empty",
            "max_rank": ranked,
        }]
    })
}

/// Judges the curated `PAIR` against a report of `clusters`.
fn judge(clusters: &[Value], max_rank: Option<u64>) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_curated_recall(
        &manifest_with_clone(&PAIR, max_rank),
        &report(clusters),
        &mut failures,
    );
    failures
}

/// A cluster spanning the curated pair in the given bucket.
fn curated(bucket: &str) -> Value {
    spanning(bucket, 1.0, 1.0, &PAIR)
}

/// A cluster spanning two other files entirely.
fn elsewhere() -> Value {
    spanning("identical", 1.0, 1.0, &["lib/other.dart", "lib/else.dart"])
}

#[test]
fn a_curated_clone_reported_shown_and_identical_passes() {
    assert!(
        judge(&[curated("identical")], None).is_empty(),
        "a shown `identical` cluster spanning the curated pair is exactly \
         what the entry asserts; a check that fires here asserts nothing"
    );
}

#[test]
fn a_missing_curated_clone_is_a_false_negative() {
    assert_only_failure(
        &judge(&[elsewhere()], None),
        "recall",
        "an unreported curated duplicate must fail",
        "lib/slider_parts.dart",
        "the detail names the missed pair",
    );
}

/// Every bucket short of `identical` that the engine can render. A
/// byte-identical pair reaching any of them is the engine contradicting a
/// verified fact about the source.
const DEMOTIONS: [&str; 4] = [
    "nearly_identical",
    "structural_only",
    "loosely_similar",
    "same_behavior",
];

#[test]
fn a_curated_clone_demoted_below_identical_is_a_quality_failure() {
    for bucket in DEMOTIONS {
        assert_only_failure(
            &judge(&[curated(bucket)], None),
            "recall_quality",
            "a byte-identical pair rendering as anything else must fail — \
             the old span-only check passed every one of these",
            bucket,
            "the detail names the bucket the pair was demoted into",
        );
    }
}

#[test]
fn a_curated_clone_with_a_hidden_occurrence_is_a_quality_failure() {
    let half_shown = json!({
        "bucket": "identical",
        "signals": { "structural": 1.0, "token_jaccard": 1.0, "embedding_cos": 0.0, "fused": 1.0 },
        "occurrences": [
            { "path": PAIR[0], "hidden": false },
            { "path": PAIR[1], "hidden": true },
        ],
    });
    assert_only_failure(
        &judge(&[half_shown], None),
        "recall_quality",
        "recall is what the report shows: a pair with one side suppressed \
         is a pair the user never sees, however the JSON is shaped",
        "lib/slider_parts.dart",
        "the detail names the pair whose side was hidden",
    );
}

#[test]
fn a_curated_clone_ranked_below_its_ceiling_is_a_quality_failure() {
    let mut clusters: Vec<Value> = (0..5).map(|_| elsewhere()).collect();
    clusters.push(curated("identical"));
    assert!(
        judge(&clusters, Some(5)).is_empty(),
        "rank 5 against a ceiling of 5 is inside it — the bound is inclusive"
    );
    assert_only_failure(
        &judge(&clusters, Some(4)),
        "recall_quality",
        "a 137-line verified clone ranking below the scaffolding is a \
         ranking defect the gate must name, not a number it prints",
        "ranks 5",
        "the detail names the rank it reached and the ceiling it broke",
    );
    assert!(
        judge(&clusters, None).is_empty(),
        "an entry with no curated ceiling asserts nothing about rank; only \
         the entries a human ranked get a rank assertion"
    );
}

#[test]
fn an_entry_naming_fewer_than_two_files_fails_rather_than_passing() {
    let mut failures = Vec::new();
    check_curated_recall(
        &manifest_with_clone(&[PAIR[0]], None),
        &report(&[curated("identical")]),
        &mut failures,
    );
    assert_eq!(
        failures.len(),
        1,
        "a one-file entry describes no duplication. It must fail as \
         uncurated, not pass by spanning trivially: {failures:?}"
    );
    assert_eq!(
        failures.first().map(|failure| failure.check.as_str()),
        Some("recall"),
        "and it fails as a recall miss, so [CORPUS-BASELINE] cannot record \
         it as a satisfied check"
    );
}
