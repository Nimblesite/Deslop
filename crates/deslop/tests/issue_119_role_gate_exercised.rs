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

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::path::Path;

use anyhow::Result;
use mock_ollama::MockOllama;
use serde_json::Value;

mod common;
use crate::common::{embeddings::run_mock_embedding_report, *};

/// `EMBEDDING_SUPPORT_FLOOR` (`crates/deslop-core/src/pair.rs`) — the
/// cosine at which embedding evidence may support a bucket at all.
const EMBEDDING_SUPPORT_FLOOR: f64 = 0.80;

/// Scans a private copy of `fixture_root` with the given deterministic
/// mock embedder wired in ([FUSION-EMBED-PROVIDER]).
fn report_with_embeddings(fixture_root: &Path, server: &MockOllama) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let scan_root = &tmp.path().join("src");
    seed(fixture_root, scan_root)?;
    run_mock_embedding_report(scan_root, &output, "5", server.endpoint())
}

/// Scans a private copy of `fixture_root` with `--embeddings off`, the
/// baseline the embedding pass must measurably move.
fn report_without_embeddings(fixture_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let scan_root = &tmp.path().join("src");
    seed(fixture_root, scan_root)?;
    run_report(scan_root, 5)
}

/// Visible clusters carrying the `same_behavior` bucket.
fn same_behavior(report: &Value) -> Vec<&Value> {
    clusters(report)
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "same_behavior")
        .collect()
}

/// Asserts the embedding pass measurably changed what the scan
/// suppressed — the only black-box proof that a pair reached the role
/// gate rather than never forming.
fn assert_gate_was_reached(fixture_name: &str, language: &str) -> Result<()> {
    let root = fixture(fixture_name);
    let without = report_without_embeddings(&root)?;
    let server = MockOllama::spawn()?;
    let with = report_with_embeddings(&root, &server)?;
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

/// Asserts a same-role, behaviour-equivalent pair surfaces with real
/// measured embedding support behind it.
fn assert_same_role_pair_surfaces(
    fixture_name: &str,
    language: &str,
    left: &str,
    right: &str,
) -> Result<()> {
    let root = fixture(fixture_name);
    let server = MockOllama::spawn_semantic(&[&[left, right]])?;
    let report = report_with_embeddings(&root, &server)?;
    let surviving = same_behavior(&report);
    assert!(
        !surviving.is_empty(),
        "[CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] two same-role behaviour-equivalent \
         {language} functions must surface as same_behavior — the role gate must \
         not over-suppress. Visible clusters: {:#?}",
        clusters(&report)
    );
    assert_pairs_both_members(&root, &surviving, left, right)?;
    assert_embedding_support(&surviving, language);
    Ok(())
}

/// Asserts one surviving cluster covers both named members.
fn assert_pairs_both_members(
    scan_root: &Path,
    surviving: &[&Value],
    left: &str,
    right: &str,
) -> Result<()> {
    let paired = surviving.iter().try_fold(false, |found, cluster| {
        let texts = occurrence_texts(scan_root, cluster)?;
        let touches_left = texts.iter().any(|text| text.contains(left));
        let touches_right = texts.iter().any(|text| text.contains(right));
        Ok::<bool, anyhow::Error>(found || (touches_left && touches_right))
    })?;
    assert!(
        paired,
        "the surviving same_behavior cluster must pair `{left}` with `{right}`: \
         {surviving:#?}"
    );
    Ok(())
}

/// Asserts every surviving `same_behavior` cluster carries the embedding
/// evidence its bucket claims.
fn assert_embedding_support(surviving: &[&Value], language: &str) {
    for cluster in surviving {
        let cos = signal(cluster, "embedding_cos");
        assert!(
            cos >= EMBEDDING_SUPPORT_FLOOR,
            "a visible {language} same_behavior cluster must carry embedding \
             support at or above {EMBEDDING_SUPPORT_FLOOR}, got {cos} on \
             {}. A same_behavior bucket without measured cosine is a bucket \
             asserted from nothing.",
            cluster_id(cluster)
        );
    }
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
    assert_same_role_pair_surfaces(
        "dart-issue-119-same-role",
        "Dart",
        "totalRecursive",
        "while (index",
    )
}

// Over-suppression guard, Python: same-role behaviour-equivalent
// functions must survive the gate with measured embedding support.
#[test]
fn python_same_role_pair_surfaces_with_measured_embedding_support() -> Result<()> {
    assert_same_role_pair_surfaces(
        "python-issue-119-same-role",
        "Python",
        "total_recursive",
        "while index",
    )
}

// Control: the `same_behavior` bucket IS reachable end-to-end when a
// same-role pair genuinely clears the embedding floor. A `while` loop and
// a `for` loop accumulating the same eight running totals share no
// normalised subtree yet measure cosine 0.91 under the shingle mock.
//
// This test passing while the four above fail localises the defect
// precisely: the pipeline and the role gate are sound, and it is the #119
// fixtures that can no longer reach the gate.
#[test]
fn same_behavior_is_reachable_when_a_pair_clears_the_embedding_floor() -> Result<()> {
    let root = fixture("dart-issue-119-same-behavior-reachable");
    let server = MockOllama::spawn()?;
    let report = report_with_embeddings(&root, &server)?;
    let surviving = same_behavior(&report);
    assert!(
        !surviving.is_empty(),
        "a same-role Dart pair measuring cosine 0.91 must surface as \
         same_behavior; if this fails the bucket is unreachable for every \
         input, not just the #119 fixtures. Visible clusters: {:#?}",
        clusters(&report)
    );
    for cluster in &surviving {
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
            "`same_behavior` means DIFFERENT code: byte-identical occurrences \
             belong in an identical bucket, not this one: {texts:#?}"
        );
        assert!(
            approx(signal(cluster, "structural"), 0.0),
            "the `while` and `for` bodies share no normalised subtree, so the \
             bucket must rest on embedding evidence alone, got structural={}",
            signal(cluster, "structural")
        );
    }
    assert_embedding_support(&surviving, "Dart");
    Ok(())
}
