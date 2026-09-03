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
        "a duplicate cluster needs at least two occurrences",
        "max(1 - 1, 0)",
        "the failure names the singleton equation",
    );
}

/// [EXCLUSION-CONFIG] A cluster whose only visible occurrence sits beside
/// `report_hide`-suppressed copies is kept intact and shown, so the user
/// sees regular code duplicating generated code. Reading the
/// two-occurrence floor off the *visible* count condemned exactly that
/// cluster: 18 of them on the Flutter corpus, every one a hand-written
/// file duplicating a `.g.dart` or generated binding, and the gate
/// demanded the engine stop publishing a finding the spec requires.
#[test]
fn a_mixed_cluster_with_one_visible_occurrence_passes() {
    let cluster = mixed("mixed", 58, &["lib/tally.dart"], &["lib/tally.g.dart"]);
    assert!(
        judge(&[cluster]).is_empty(),
        "one visible occurrence beside a hidden generated copy is a \
         published finding, not a contract breach"
    );
}

/// The floor still bites when the cluster really is alone: one carried
/// occurrence is not duplication however it is rendered.
#[test]
fn a_cluster_carrying_one_occurrence_fails_even_when_it_is_visible() {
    let cluster = mixed("lonely", 58, &["lib/tally.dart"], &[]);
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_mass",
        "a single carried occurrence is not a duplicate",
        "1 carried and 1 visible occurrences",
        "the failure separates the carried count from the visible one",
    );
}

/// Hiding does not license a wrong equation: mass follows the visible
/// count, so a mixed cluster claiming mass for its hidden copies fails.
#[test]
fn a_mixed_cluster_may_not_claim_mass_for_hidden_copies() {
    let mut cluster = mixed("greedy", 58, &["lib/tally.dart"], &["lib/tally.g.dart"]);
    let _old = cluster
        .as_object_mut()
        .and_then(|fields| fields.insert("mass".to_owned(), json!(58)));
    assert_only_failure(
        &judge(&[cluster]),
        "cluster_mass",
        "mass counts visible occurrences only",
        "58 × max(1 - 1, 0) = 0",
        "the failure prints the visible-count equation",
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
