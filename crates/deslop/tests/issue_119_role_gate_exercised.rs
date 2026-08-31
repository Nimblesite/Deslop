//! [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] — the role gate must be
//! *exercised*, not merely present.
//!
//! GH #119 added a role-compatibility gate: an embedding-dominant
//! `same_behavior` cluster must be role/context compatible (all classes,
//! or all functions) before surfacing. Two suites assert that gate — the
//! Dart and Python `issue_119_embedding_role_mismatch` binaries — and
//! both of their suppression tests are written as "no offending cluster
//! is visible".
//!
//! That shape passes for two different reasons, and only one of them is
//! the gate working:
//!
//! 1. the embedding pass paired the role-incompatible members and the
//!    gate suppressed the cluster — the contract, or
//! 2. the embedding pass never paired them at all, so no cluster ever
//!    existed to suppress and the assertion is vacuous.
//!
//! This binary pins the difference between the two reasons so it cannot
//! be mistaken for a passing gate again. The role-mismatch fixtures are
//! near-identical byte-for-byte (a class and a function sharing locals),
//! so the honest GH #369 shingle mock scores them above the floor and the
//! pair genuinely reaches the gate. The same-role fixtures are genuinely
//! Type-4 — same behaviour, different text — which no content statistic
//! can measure, so those tests declare the ground truth with
//! [`MockOllama::spawn_semantic`]: the two function names form one
//! behaviour-equivalence group, lifting their cosine above the floor
//! while every other pair keeps its honest shingle cosine.

use std::path::Path;

use crate::mock_ollama::MockOllama;
use anyhow::Result;
use serde_json::Value;

use crate::common::{
    embeddings::scan_fixture_copy_with_mock,
    role_gate::*,
    signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract},
    *,
};

/// Scans a private copy of `fixture_root` with `--embeddings off`, the
/// baseline the embedding pass must measurably move.
fn report_without_embeddings(fixture_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let scan_root = &tmp.path().join("src");
    seed(fixture_root, scan_root)?;
    run_report(scan_root, 5)
}

/// Asserts the embedding pass measurably changed what the scan
/// suppressed — the only black-box proof that a pair reached the role
/// gate rather than never forming.
fn assert_gate_was_reached(fixture_name: &str, language: &str) -> Result<()> {
    let root = fixture(fixture_name);
    let without = report_without_embeddings(&root)?;
    let server = MockOllama::spawn()?;
    let with = scan_fixture_copy_with_mock(&root, "5", server.endpoint())?;
    let hidden_without = clusters_hidden(&without);
    let hidden_with = clusters_hidden(&with);
    assert!(
        hidden_with > hidden_without,
        "[CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] the {language} role-mismatch \
         fixture `{fixture_name}` must form a role-incompatible embedding pair \
         that the gate then suppresses: clusters_hidden must RISE when the \
         embedding pass runs, got {hidden_without} with `--embeddings off` and \
         {hidden_with} with the mock embedder. Equal counts mean the ANN pass \
         emitted zero pairs, no cluster was ever built, and the suppression \
         assertion in the {language} #119 suite is vacuous — it would pass \
         with the role gate deleted."
    );
    Ok(())
}

// The Dart role-mismatch fixture must actually build the class/function
// pair the gate exists to suppress.
#[test]
fn dart_role_mismatch_pair_must_reach_the_role_gate() -> Result<()> {
    assert_gate_was_reached("dart-issue-119-role-mismatch", "Dart")
}

// The Python role-mismatch fixture must actually build the class/function
// pair the gate exists to suppress.
#[test]
fn python_role_mismatch_pair_must_reach_the_role_gate() -> Result<()> {
    assert_gate_was_reached("python-issue-119-role-mismatch", "Python")
}

// Over-suppression guard, Dart: same-role behaviour-equivalent functions
// must survive the gate with measured embedding support.
#[test]
fn dart_same_role_pair_surfaces_with_measured_embedding_support() -> Result<()> {
    let server = MockOllama::spawn_semantic(&[&["totalRecursive", "totalIterative"]])?;
    assert_same_role_pair_surfaces(
        "dart-issue-119-same-role",
        "Dart",
        server.endpoint(),
        "totalRecursive",
        "while (index",
    )
}

// Over-suppression guard, Python: same-role behaviour-equivalent
// functions must survive the gate with measured embedding support.
#[test]
fn python_same_role_pair_surfaces_with_measured_embedding_support() -> Result<()> {
    let server = MockOllama::spawn_semantic(&[&["total_recursive", "total_iterative"]])?;
    assert_same_role_pair_surfaces(
        "python-issue-119-same-role",
        "Python",
        server.endpoint(),
        "total_recursive",
        "while index",
    )
}

// Control: a same-role pair that genuinely clears the embedding floor
// must stay VISIBLE. A `while` loop and a `for` loop accumulating the
// same eight running totals are duplicated code by any reading, and the
// report must say so.
//
// This asserted the `same_behavior` bucket specifically, and that is no
// longer the honest label for this pair. Measured: `structural = 0.912`,
// `embedding_cos = 0.911`, `token_jaccard = 0.555`. The shape carries
// this match at least as strongly as the model does, so
// [CLONE-BUCKETS-ROUTING] row 2 — semantic evidence *without* shape —
// does not describe it. The old expectation only held because
// `structural` was Merkle equality: the differing loop keyword rehashed
// the root, so a pair sharing 91% of its AST measured exactly 0.0 and
// fell into row 2 by default ([FUSED-SHARED-SUBTREE], gh #408).
//
// `same_behavior` reachability is still asserted end-to-end, on the two
// fixtures that are genuinely Type-4 — recursion against iteration —
// by `dart_same_role_pair_surfaces_with_measured_embedding_support` and
// its Python twin above. Both call `assert_same_role_pair_surfaces`,
// which requires a surviving `same_behavior` cluster. No coverage of
// that property is lost here.
//
// What this test guards is the false negative, and it guards it harder
// than before: the pair must be visible at all. It was not — until row
// 4b learned to accept embedding corroboration, two independent signals
// agreeing at 0.91 routed `loosely_similar`, which the renderer hides,
// on the strength of a 0.55 token score.
#[test]
fn same_role_pair_clearing_the_embedding_floor_stays_visible() -> Result<()> {
    let root = fixture("dart-issue-119-same-behavior-reachable");
    let server = MockOllama::spawn()?;
    let report = scan_fixture_copy_with_mock(&root, "5", server.endpoint())?;
    let surviving = clusters(&report);
    assert!(
        !surviving.is_empty(),
        "a same-role Dart pair measuring cosine 0.91 against structural 0.91 must \
         surface: two independent signals agree that these accumulate identically, \
         and hiding it is a false negative. Report: {report:#}"
    );
    for cluster in surviving {
        let texts = occurrence_texts(&root, cluster)?;
        assert_eq!(
            texts.len(),
            2,
            "the accumulator clone pairs exactly the two loop bodies: {texts:#?}"
        );
        assert!(
            texts
                .iter()
                .all(|text| text.contains("stacked = stacked + carried;")),
            "both occurrences must carry the shared accumulator chain — that \
             is the behaviour they have in common: {texts:#?}"
        );
        assert!(
            texts.first() != texts.get(1),
            "the occurrences must be DIFFERENT code: byte-identical occurrences \
             belong in an identical bucket, not a near-miss one: {texts:#?}"
        );
        // [PIPELINE-CLUSTER-CLOSURE] The shape/embedding evidence is
        // pair-scoped now; the wire facts that hold the acceptance: the
        // near-miss is admitted, mass-honest and clean-surfaced, and its
        // occurrences are byte-distinct (already asserted above).
        assert_structural_only_contract(cluster, "dart #119 role gate");
        assert_no_pair_surface_on_cluster(cluster, "dart #119 role gate");
    }
    Ok(())
}
