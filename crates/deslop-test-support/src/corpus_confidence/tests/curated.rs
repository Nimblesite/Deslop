//! Curated Type-2 recall asserts membership and extent, never cluster evidence.

use super::*;

/// Curated pair used by every case.
const PAIR: [&str; 2] = ["src/a.ts", "src/b.ts"];
/// Minimum normalised extent curated for the pair.
const MIN_NODES: u64 = 300;

/// Manifest containing one hand-verified renamed pair.
fn manifest(min_nodes: Option<u64>) -> Value {
    json!({
        "must_find_type2": [{
            "files": PAIR,
            "min_nodes": min_nodes,
            "why": "hand-verified renamed module pair"
        }]
    })
}

/// Runs the curated Type-2 assertion.
fn judge(clusters: &[Value], min_nodes: Option<u64>) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_type2_curated_recall(&manifest(min_nodes), &report(clusters), &mut failures);
    failures
}

#[test]
fn reported_pair_at_curated_extent_passes() {
    let cluster = spanning("pair", MIN_NODES, 1, &PAIR);
    assert!(judge(&[cluster], Some(MIN_NODES)).is_empty());
}

#[test]
fn missing_curated_pair_fails() {
    let cluster = spanning("elsewhere", MIN_NODES, 1, &["src/c.ts", "src/d.ts"]);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "a missing curated pair is a false negative",
        "src/a.ts",
        "the failure names the missing pair",
    );
}

#[test]
fn fragment_below_curated_extent_fails() {
    let cluster = spanning("fragment", MIN_NODES.saturating_sub(1), 1, &PAIR);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "a fragment is not the curated module clone",
        "expected at least 300",
        "the failure names the extent deficit",
    );
}

#[test]
fn missing_extent_curation_fails() {
    assert_only_failure(
        &judge(&[spanning("pair", MIN_NODES, 1, &PAIR)], None),
        "type2_recall",
        "a manifest without extent asserts too little",
        "min_nodes",
        "the failure names the missing field",
    );
}

#[test]
fn hidden_curated_occurrence_fails() {
    let cluster = hide_occurrence(spanning("pair", MIN_NODES, 1, &PAIR), PAIR[1]);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "both curated sides must be visible",
        "src/a.ts",
        "the failure names the pair",
    );
}

#[test]
fn unrelated_sprawl_below_extent_does_not_satisfy_recall() {
    let files = [PAIR[0], PAIR[1], "src/net.ts", "src/process.ts"];
    let cluster = spanning("boilerplate", 31, 1, &files);
    assert_only_failure(
        &judge(&[cluster], Some(MIN_NODES)),
        "type2_recall",
        "path overlap alone cannot satisfy curated recall",
        "31 canonical nodes",
        "the failure names the unrelated small extent",
    );
}

#[test]
fn extent_above_floor_passes() {
    let cluster = spanning("pair", MIN_NODES.saturating_add(80), 1, &PAIR);
    assert!(judge(&[cluster], Some(MIN_NODES)).is_empty());
}

#[test]
fn empty_curated_list_asserts_nothing() {
    let mut failures = Vec::new();
    check_type2_curated_recall(&json!({"must_find_type2": []}), &report(&[]), &mut failures);
    assert!(failures.is_empty());
}

/// A ceiling one place above the curated pair's rank in [`buried_pair`].
const CEILING: u64 = 1;

/// The curated pair at its extent, ranked below one unrelated clone.
fn buried_pair() -> [Value; 2] {
    [
        spanning("elsewhere", MIN_NODES, 1, &["src/c.ts", "src/d.ts"]),
        spanning("pair", MIN_NODES, 2, &PAIR),
    ]
}

/// Runs the Type-2 assertion against a manifest that also curates `max_rank`.
fn judge_with_ceiling(clusters: &[Value], ceiling: u64) -> Vec<Failure> {
    let mut entry = manifest(Some(MIN_NODES));
    if let Some(fields) = entry
        .pointer_mut("/must_find_type2/0")
        .and_then(Value::as_object_mut)
    {
        let _previous = fields.insert("max_rank".to_owned(), json!(ceiling));
    }
    let mut failures = Vec::new();
    check_type2_curated_recall(&entry, &report(clusters), &mut failures);
    failures
}

/// gh #439: a curated Type-2 module reported whole but buried below its
/// curated rank ceiling fails, as a `must_find` entry does.
#[test]
fn a_type2_pair_ranked_below_its_ceiling_fails() {
    assert_only_failure(
        &judge_with_ceiling(&buried_pair(), CEILING),
        "recall_quality",
        "a whole module buried below its curated ceiling is not reported where it belongs",
        "ranks 2 below curated ceiling 1",
        "the failure names the rank and the ceiling",
    );
}

#[test]
fn a_type2_pair_within_its_ceiling_passes() {
    assert!(judge_with_ceiling(&buried_pair(), CEILING.saturating_add(1)).is_empty());
    assert!(
        judge(&buried_pair(), Some(MIN_NODES)).is_empty(),
        "no ceiling curated, none applied"
    );
}
