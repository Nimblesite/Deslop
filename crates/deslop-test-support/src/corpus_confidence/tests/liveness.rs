//! Exhaustive mass-only cluster contract assertions.

use super::*;

/// Runs the cluster contract check.
fn judge(clusters: &[Value]) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_cluster_mass_contract(&report(clusters), &mut failures);
    failures
}

#[test]
fn valid_mass_only_cluster_passes() {
    let cluster = spanning("valid", 40, 1, &["a.rs", "b.rs", "c.rs"]);
    assert!(judge(&[cluster]).is_empty());
}

#[test]
fn pair_evidence_on_a_cluster_fails() {
    let mut cluster = spanning("leak", 40, 1, &["a.rs", "b.rs"]);
    let _old = cluster
        .as_object_mut()
        .and_then(|fields| fields.insert("signals".to_owned(), json!({"structural": 1.0})));
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_contract",
        "pair evidence must never leak onto a cluster",
        "signals",
        "the failure names the leaked field",
    );
}

#[test]
fn cluster_weight_alias_fails() {
    let mut cluster = spanning("leak", 40, 1, &["a.rs", "b.rs"]);
    let _old = cluster
        .as_object_mut()
        .and_then(|fields| fields.insert("weight".to_owned(), json!(40)));
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_contract",
        "mass has no weight alias",
        "weight",
        "the failure names the alias",
    );
}

#[test]
fn classification_on_a_cluster_fails() {
    let mut cluster = spanning("leak", 40, 1, &["a.rs", "b.rs"]);
    let _old = cluster
        .as_object_mut()
        .and_then(|fields| fields.insert("classification".to_owned(), json!("identical")));
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_contract",
        "classification belongs only to an exact pair",
        "classification",
        "the failure names the leak",
    );
}

#[test]
fn incorrect_mass_fails() {
    let mut cluster = spanning("bad-mass", 40, 1, &["a.rs", "b.rs", "c.rs"]);
    let _old = cluster
        .as_object_mut()
        .and_then(|fields| fields.insert("mass".to_owned(), json!(79)));
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_mass",
        "mass must equal extent times additional occurrences",
        "expected 40 × max(3 - 1, 0) = 80",
        "the failure prints the exact equation",
    );
}

#[test]
fn single_occurrence_cluster_fails() {
    let cluster = spanning("singleton", 40, 1, &["a.rs"]);
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_mass",
        "a duplicate cluster needs at least two visible members",
        "max(1 - 1, 0)",
        "the failure names the singleton equation",
    );
}

#[test]
fn nonsequential_rank_fails() {
    let cluster = spanning("bad-rank", 40, 2, &["a.rs", "b.rs"]);
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_rank",
        "rank must equal report position",
        "position 1 carries rank 2",
        "the failure names both positions",
    );
}

#[test]
fn sequential_multi_cluster_ranks_pass() {
    let first = spanning("first", 50, 1, &["a.rs", "b.rs"]);
    let second = spanning("second", 20, 2, &["c.rs", "d.rs"]);
    assert!(judge(&[first, second]).is_empty());
}
