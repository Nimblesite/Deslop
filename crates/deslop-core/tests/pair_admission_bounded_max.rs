//! Pair-layer admission pins for [FUSION-STRATEGY-BOUNDED-MAX] (gh #343).
//!
//! The rendered-report inequality (`fused <= max(axes)`, enforced by the
//! corpus `fused_bounded_max` check) cannot prove the *admission*
//! arithmetic: where the strongest axis saturates, sum-then-clamp and
//! bounded max render the same number, and scheduled embeddings-off
//! corpora rewrite shape clusters through the content gate before any
//! report is compared. The only place the two formulas provably diverge
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
/// pair. Only the signal triple varies across the tests.
fn candidate(structural: f64, token_jaccard: f64, embedding_cos: f64) -> CandidatePair {
    CandidatePair {
        left: 0,
        right: 1,
        endpoint_node_counts: (LSH_ONLY_MIN_NODE_COUNT, LSH_ONLY_MIN_NODE_COUNT),
        lsh_only_node_floor: LSH_ONLY_MIN_NODE_COUNT,
        lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
        fused_min_score: FUSED_THRESHOLD,
        shared_subtree_overlap: 0.0,
        score: PairScore {
            structural,
            token_jaccard,
            embedding_cos,
        },
    }
}

// [FUSION-STRATEGY-BOUNDED-MAX] The discriminating triple: every axis is
// below the 0.85 threshold, so bounded max computes 0.44 and must drop
// the pair, while the quarantined sum-then-clamp arm computes
// 0.44 + 0.42 + 0.0 = 0.86 and would admit it. A cluster appearing here
// is the #343 revert reaching production admission.
#[test]
fn sub_threshold_axes_are_dropped_even_though_their_sum_clears_the_bar() {
    // Fixture arithmetic, fixed by construction: every axis (0.44,
    // 0.42, 0.0) sits below the 0.85 threshold while their sum, 0.86,
    // clears it — the one shape that tells the two formulas apart.
    let clusters = cluster_by_transitive_closure(&[candidate(0.44, 0.42, 0.0)]);
    assert!(
        clusters.is_empty(),
        "a pair whose strongest axis is 0.44 must be DroppedBelowFused under bounded max — \
         admitting it means the confidence was summed across correlated axes (gh #343): \
         {clusters:?}"
    );
}

// The positive control: one axis alone above the threshold must survive,
// producing exactly one two-member component. Without this, the test
// above would also pass if admission dropped everything.
#[test]
fn a_single_axis_above_the_threshold_survives_admission() {
    let clusters = cluster_by_transitive_closure(&[candidate(0.86, 0.0, 0.0)]);
    assert_eq!(
        clusters.len(),
        1,
        "a structural 0.86 pair clears the 0.85 floor on its own: {clusters:?}"
    );
    let members: Vec<usize> = clusters
        .first()
        .map(|cluster| cluster.members.clone())
        .unwrap_or_default();
    assert_eq!(
        members,
        vec![0, 1],
        "and the component must contain exactly the pair's two endpoints"
    );
}

// The threshold is a floor, not an open bound: exactly 0.85 survives.
// Pinned separately because an accidental `<=` in the drop test flips
// only this case, never the two above.
#[test]
fn an_axis_exactly_at_the_threshold_survives_admission() {
    let clusters = cluster_by_transitive_closure(&[candidate(FUSED_THRESHOLD, 0.0, 0.0)]);
    assert_eq!(
        clusters.len(),
        1,
        "an axis exactly at {FUSED_THRESHOLD} must not be dropped: {clusters:?}"
    );
}
