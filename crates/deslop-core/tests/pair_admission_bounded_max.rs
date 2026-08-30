//! Pair-layer admission pins for [FUSED-STRATEGY-BOUNDED-MAX] (gh #343).
//!
//! A rendered report no longer exposes the admission-only `fused` quantity,
//! so only the pair gate can prove the admission arithmetic. The place the
//! two formulas provably diverge
//! is a pair whose axes all sit **below** the threshold while their sum
//! clears it — so that pair is pinned here, at the production survival
//! gate itself, driven through the public clustering entry point.

use deslop_core::pair::{
    cluster_by_transitive_closure, CandidatePair, PairScore, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD,
    LSH_ONLY_MIN_NODE_COUNT,
};

/// A same-language candidate pair between two healthy, size-coherent
/// endpoints, carrying the default admission floors — the exact shape
/// `finalise_pairs` hands to `survival_decision` for an ordinary corpus
/// pair. `exact_merkle` is a boolean because admission evidence `H` is an
/// indicator, never a fractional structural-overlap score.
fn candidate(exact_merkle: bool, token_jaccard: f64, embedding_cos: f64) -> CandidatePair {
    CandidatePair {
        left: LEFT_INDEX,
        right: RIGHT_INDEX,
        endpoint_node_counts: (LSH_ONLY_MIN_NODE_COUNT, LSH_ONLY_MIN_NODE_COUNT),
        lsh_only_node_floor: LSH_ONLY_MIN_NODE_COUNT,
        lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
        fused_min_score: FUSED_THRESHOLD,
        shared_subtree_overlap: 0.0,
        score: PairScore {
            structural: if exact_merkle { 1.0 } else { 0.0 },
            token_jaccard,
            embedding_cos,
        },
    }
}

/// Strongest sub-threshold axis in the discriminating pair.
const SUB_THRESHOLD_TOKEN: f64 = 0.44;
/// Corroborating sub-threshold embedding axis that makes the forbidden sum clear the bar.
const SUB_THRESHOLD_EMBEDDING: f64 = 0.42;
/// One independently sufficient axis above the admission bar.
const ABOVE_THRESHOLD_AXIS: f64 = 0.86;
/// No evidence on an axis.
const ABSENT_AXIS: f64 = 0.0;
/// Left endpoint of every synthetic pair.
const LEFT_INDEX: usize = 0;
/// Right endpoint of every synthetic pair.
const RIGHT_INDEX: usize = 1;
/// Exact component membership expected after admission.
const EXPECTED_MEMBERS: [usize; 2] = [LEFT_INDEX, RIGHT_INDEX];

// [FUSED-STRATEGY-BOUNDED-MAX] The discriminating triple: every axis is
// below the 0.85 threshold, so bounded max computes 0.44 and must drop
// the pair, while the quarantined sum-then-clamp arm computes
// 0.44 + 0.42 + 0.0 = 0.86 and would admit it. A cluster appearing here
// is the #343 revert reaching production admission.
#[test]
fn sub_threshold_axes_are_dropped_even_though_their_sum_clears_the_bar() {
    // Fixture arithmetic, fixed by construction: H=0, J=0.44, and E=0.42
    // all sit below the 0.85 threshold while J+E=0.86 clears it.
    let clusters = cluster_by_transitive_closure(&[candidate(
        false,
        SUB_THRESHOLD_TOKEN,
        SUB_THRESHOLD_EMBEDDING,
    )]);
    assert!(
        clusters.is_empty(),
        "a pair whose strongest axis is {SUB_THRESHOLD_TOKEN} must be DroppedBelowFused under bounded max — \
         admitting it means the confidence was summed across correlated axes (gh #343): \
         {clusters:?}"
    );
}

// The positive control: one axis alone above the threshold must survive,
// producing exactly one two-member component. Without this, the test
// above would also pass if admission dropped everything.
#[test]
fn a_single_axis_above_the_threshold_survives_admission() {
    let clusters =
        cluster_by_transitive_closure(&[candidate(false, ABSENT_AXIS, ABOVE_THRESHOLD_AXIS)]);
    assert_eq!(
        clusters.len(),
        1,
        "an embedding axis at {ABOVE_THRESHOLD_AXIS} clears the {FUSED_THRESHOLD} floor on its own: {clusters:?}"
    );
    let members: Vec<usize> = clusters
        .first()
        .map(|cluster| cluster.members.clone())
        .unwrap_or_default();
    assert_eq!(
        members, EXPECTED_MEMBERS,
        "and the component must contain exactly the pair's two endpoints"
    );
}

// The threshold is a floor, not an open bound: exactly 0.85 survives.
// Pinned separately because an accidental `<=` in the drop test flips
// only this case, never the two above.
#[test]
fn an_axis_exactly_at_the_threshold_survives_admission() {
    let clusters = cluster_by_transitive_closure(&[candidate(false, ABSENT_AXIS, FUSED_THRESHOLD)]);
    assert_eq!(
        clusters.len(),
        1,
        "an axis exactly at {FUSED_THRESHOLD} must not be dropped: {clusters:?}"
    );
}
