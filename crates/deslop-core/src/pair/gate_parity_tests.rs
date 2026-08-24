//! [PERF-FLUTTER-TODO-PAIRS] The insertion-time construction gate is
//! a performance rewrite of `survival_decision` at overlap 0 — it
//! must refuse exactly the pairs the closure would drop and keep
//! exactly the ones it would keep. These pins drive both functions
//! over a matrix of signal triples, floors, and endpoint shapes so
//! the two halves of the decision can never drift
//! (`docs/performance-branch-review.md`, "admission parity").

use super::{
    construction_survives, survival_decision, CandidatePair, PairScore, PairSurvival,
    FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT,
};

/// One matrix cell: the pair shape, built with an overlap of 0 —
/// the construction gate runs before the rescue measures anything.
fn pair(
    structural: f64,
    token_jaccard: f64,
    embedding_cos: f64,
    coherent: bool,
    node_floor: usize,
    min_jaccard: f64,
    fused_min: f64,
) -> CandidatePair {
    CandidatePair {
        left: 0,
        right: 1,
        endpoint_node_counts: if coherent {
            (LSH_ONLY_MIN_NODE_COUNT, LSH_ONLY_MIN_NODE_COUNT)
        } else {
            (3, 3 * LSH_ONLY_MIN_NODE_COUNT)
        },
        lsh_only_node_floor: node_floor,
        lsh_only_min_jaccard: min_jaccard,
        fused_min_score: fused_min,
        shared_subtree_overlap: 0.0,
        score: PairScore {
            structural,
            token_jaccard,
            embedding_cos,
        },
    }
}

/// The parity invariant: construction keeps exactly the pairs the
/// closure survives at zero overlap. Rescue-eligible pairs may be
/// refused by construction and re-admitted by measurement — that is
/// `rescue_eligible`'s contract, asserted separately below.
#[test]
fn construction_gate_matches_survival_at_zero_overlap() {
    let structural_axis = [1.0, 0.0];
    let jaccard_axis = [0.95, 0.80, 0.50];
    let embedding_axis = [0.9, 0.0];
    let coherence_axis = [true, false];
    let floor_axis = [LSH_ONLY_MIN_NODE_COUNT, 5];
    let min_jaccard_axis = [LSH_ONLY_MIN_JACCARD, 0.60];
    let fused_axis = [FUSED_THRESHOLD, 0.60];
    let mut cells = 0_u32;
    for &structural in &structural_axis {
        for &jaccard in &jaccard_axis {
            for &embedding in &embedding_axis {
                for &coherent in &coherence_axis {
                    for &floor in &floor_axis {
                        for &min_jaccard in &min_jaccard_axis {
                            for &fused_min in &fused_axis {
                                let candidate = pair(
                                    structural,
                                    jaccard,
                                    embedding,
                                    coherent,
                                    floor,
                                    min_jaccard,
                                    fused_min,
                                );
                                let survives = matches!(
                                    survival_decision(&candidate),
                                    PairSurvival::Survived
                                );
                                assert_eq!(
                                    construction_survives(&candidate),
                                    survives,
                                    "construction gate and closure disagree on \
                                     structural={structural}, jaccard={jaccard}, \
                                     embedding={embedding}, coherent={coherent}, \
                                     floor={floor}, min_jaccard={min_jaccard}, \
                                     fused_min={fused_min}"
                                );
                                cells = cells.saturating_add(1);
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cells, 192, "the matrix must exercise every planned cell");
}

/// A pair the construction gate refuses on its fused floor is still
/// rescue-eligible exactly when the rescue route's own contract
/// holds: zero structural anchor, corroborating token Jaccard, and
/// a substantive smaller endpoint.
#[test]
fn refused_pairs_reenter_only_through_the_rescue_route() {
    let mut eligible = pair(
        0.0,
        super::SHARED_SUBTREE_MIN_JACCARD,
        0.0,
        true,
        LSH_ONLY_MIN_NODE_COUNT,
        LSH_ONLY_MIN_JACCARD,
        FUSED_THRESHOLD,
    );
    assert!(
        !construction_survives(&eligible)
            && super::rescue_eligible(&eligible),
        "a below-floor, token-corroborated pair must be refused at \
         construction and admitted to measurement"
    );
    eligible.score.token_jaccard = 0.5;
    assert!(
        !super::rescue_eligible(&eligible),
        "weak token corroboration must close the rescue route"
    );
}
