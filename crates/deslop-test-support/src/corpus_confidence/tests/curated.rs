//! [CORPUS-RECALL] curated `type2_recall` — every hand-verified rename in
//! a manifest's `must_find_type2` list must be reported, shown and vouched.

use super::*;

/// The pair every case below curates.
const PAIR: [&str; 2] = ["src/a.ts", "src/b.ts"];

/// gh #439 witness 1 — the extent of the `as_raw_fd` accessor family
/// that satisfied this check on tokio while the curated 134-line module
/// rename was absent from the report entirely.
const ACCESSOR_NODES: u64 = 31;

/// gh #439 witness 2 — the extent of the fragment tokio reported for
/// the curated pair at rank 1628 of 2155, before the
/// [PIPELINE-CLUSTER-ELECT] weld fix moved the whole-module view to
/// rank 78. The check was green on both.
const FRAGMENT_NODES: u64 = 39;

/// The unrelated files the accessor family also spans. Five lines of
/// platform boilerplate repeated across a repository reach the curated
/// pair's two files *and* a dozen others; the curated rename reaches
/// only its own. Spanning is not the same claim as being the duplicate.
const SPRAWL: [&str; 3] = ["src/net/tcp.ts", "src/net/udp.ts", "src/process/unix.ts"];

/// Judges the curated `PAIR` against a report of `clusters`.
fn judge(clusters: &[Value]) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_type2_curated_recall(
        &manifest_with_type2(&PAIR),
        &report(clusters),
        &mut failures,
    );
    failures
}

/// Judges the curated `PAIR` against a report of one cluster that spans
/// `files` with the given bucket and signals.
fn judge_spanning(bucket: &str, structural: f64, token: f64, files: &[&str]) -> Vec<Failure> {
    judge(&[spanning(bucket, structural, token, files)])
}

#[test]
fn a_curated_type2_pair_reported_and_vouched_passes() {
    let failures = judge_spanning("nearly_identical", 1.0, 1.0, &PAIR);
    assert!(
        failures.is_empty(),
        "a visible gate-vouched cluster spanning the curated pair is recall: {failures:?}"
    );
}

#[test]
fn a_missing_curated_type2_pair_is_a_false_negative() {
    let elsewhere = ["src/other.ts", "src/else.ts"];
    let failures = judge_spanning("nearly_identical", 1.0, 1.0, &elsewhere);
    assert_only_failure(
        &failures,
        "type2_recall",
        "an unreported curated pair must fail",
        "src/a.ts",
        "the detail names the missed pair",
    );
}

#[test]
fn a_curated_pair_found_but_demoted_is_a_gate_failure() {
    // The cluster exists and spans the pair, but the gate demoted it — the
    // user is told to verify scaffolding instead of acting on a proven
    // rename. Recall is about what the report *claims*, not what it lists.
    let failures = judge_spanning("structural_only", 1.0, 0.3, &PAIR);
    assert_only_failure(
        &failures,
        "type2_recall",
        "a demoted curated pair must fail",
        "demoted",
        "the detail says the gate failed to vouch",
    );
}

#[test]
fn a_curated_pair_in_a_sub_floor_near_miss_is_not_vouched_recall() {
    // Spanning the pair in a `nearly_identical` cluster the gate never
    // judged (sub-floor token, unsaturated structural) is the same false
    // green the liveness check rejects, scoped to one entry.
    let failures = judge_spanning("nearly_identical", 0.0, 0.93, &PAIR);
    assert_eq!(failures.len(), 1, "an unvouched curated pair must fail");
    assert_eq!(only_check(&failures), Some("type2_recall"));
}

#[test]
fn a_curated_pair_whose_own_occurrence_is_hidden_is_not_recall() {
    // The cluster renders — a sibling occurrence is shown — so every
    // cluster-level visibility test passes. But the curated file's own side
    // is suppressed, so the user never sees the pair the entry proves is
    // there. Recall is what the report *shows*, not what it contains.
    let failures = judge(&[with_hidden(&["src/a.ts", "src/c.ts"], &["src/b.ts"])]);
    assert_only_failure(
        &failures,
        "type2_recall",
        "a suppressed curated side must fail",
        "hidden",
        "the detail must name suppression as a cause",
    );
}

/// A gate-vouched cluster whose occurrences are split between shown and
/// hidden, for the one case cluster-level visibility cannot express.
fn with_hidden(shown: &[&str], hidden: &[&str]) -> Value {
    let occurrences: Vec<Value> = shown
        .iter()
        .map(|file| json!({ "path": file, "hidden": false }))
        .chain(
            hidden
                .iter()
                .map(|file| json!({ "path": file, "hidden": true })),
        )
        .collect();
    json!({
        "bucket": "nearly_identical",
        "signals": { "structural": 1.0, "token_jaccard": 1.0, "embedding_cos": 0.0, "fused": 1.0 },
        "occurrences": occurrences,
    })
}

#[test]
fn an_empty_curated_list_asserts_nothing() {
    let clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    let mut failures = Vec::new();
    check_type2_curated_recall(&json!({}), &report(&clusters), &mut failures);
    check_type2_curated_recall(
        &json!({ "must_find_type2": [] }),
        &report(&clusters),
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "no curated entries means no recall assertion, pass or fail: {failures:?}"
    );
}

#[test]
fn a_boilerplate_family_spanning_the_curated_pair_is_not_the_curated_rename() {
    // gh #439. Measured on tokio at 7bb29d4: delete the curated 395-node
    // module rename from the report and this check stays green, satisfied
    // by a 31-node `as_raw_fd` accessor family spanning both curated files
    // and eleven unrelated `net/`/`process/` ones. The curated claim is
    // "the two standard-stream handles are one module written twice"; five
    // lines of platform boilerplate is not evidence for it. A check that
    // passes with its ground truth deleted asserts nothing.
    let sprawl: Vec<&str> = PAIR.iter().chain(SPRAWL.iter()).copied().collect();
    let accessor = sized(
        spanning("nearly_identical", 1.0, 1.0, &sprawl),
        ACCESSOR_NODES,
    );
    assert_only_failure(
        &judge(&[accessor.clone()]),
        "type2_recall",
        "a boilerplate family far below the curated extent is not the curated rename",
        "extent",
        "the detail must say the reported cluster is too small to be the curated duplicate",
    );

    // Same paths, same bucket, same signals, same visibility — extent is
    // the only variable, so the case cannot pass for an unrelated reason.
    let module = sized(accessor, CURATED_MIN_NODES);
    assert!(
        judge(&[module]).is_empty(),
        "the identical cluster at the curated extent is recall: only the extent may decide this"
    );
}

#[test]
fn a_fragment_far_below_the_curated_extent_is_not_the_curated_rename() {
    // gh #439 witness 2. At 7332719 tokio reported the curated pair as a
    // 39-node fragment ranked 1628 of 2155 — a finding no user scrolls to
    // — and this check was green. One commit later the same pair is the
    // 348-node whole-module view. Recall that cannot tell those apart is
    // not measuring recall.
    let fragment = sized(spanning("nearly_identical", 1.0, 1.0, &PAIR), FRAGMENT_NODES);
    assert_only_failure(
        &judge(&[fragment]),
        "type2_recall",
        "a fragment of the curated duplicate is not the curated duplicate",
        "extent",
        "the detail must say the reported cluster is too small to be the curated duplicate",
    );

    assert!(
        judge(&[spanning("nearly_identical", 1.0, 1.0, &PAIR)]).is_empty(),
        "the whole-module view of the same pair is recall"
    );
}

#[test]
fn an_entry_curating_no_extent_asserts_nothing_and_must_fail() {
    // [CORPUS-RECALL] the same stance [CORPUS-SCOPE] takes on a missing
    // `expect_files_min`: an entry that curates no extent cannot tell the
    // module from the fragment, so it must fail rather than pass on the
    // strength of a path overlap. Otherwise gh #439 reopens itself the
    // next time a manifest adds an entry.
    let uncurated = json!({
        "must_find_type2": [{
            "files": PAIR,
            "why": "hand-verified rename pair that forgot to curate its extent",
        }]
    });
    let mut failures = Vec::new();
    check_type2_curated_recall(
        &uncurated,
        &report(&[spanning("nearly_identical", 1.0, 1.0, &PAIR)]),
        &mut failures,
    );
    assert_only_failure(
        &failures,
        "type2_recall",
        "an entry curating no extent must fail, not pass vacuously",
        "min_nodes",
        "the detail must name the missing curation",
    );
}
