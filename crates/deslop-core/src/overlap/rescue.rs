//! Applies the shared-subtree rescue over the candidate set
//! ([FUSION-SHARED-SUBTREE], gh #408).
//!
//! The loop is corpus-scale — tens of millions of scanned candidates on
//! a large repository — so its observability contract is the aggregate
//! gate counters in [`super::tally`] plus fixed-interval progress
//! records, never per-pair events
//! ([PERF-FLUTTER-TODO-OBSERVABILITY]).

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    pair::{
        CandidatePair, SHARED_SUBTREE_MIN_JACCARD, SHARED_SUBTREE_MIN_NODE_COUNT,
        SHARED_SUBTREE_MIN_OVERLAP,
    },
};

use super::{tally::RescueTally, OverlapMeasurer};

/// Measures shared-subtree overlap onto every candidate pair the fused
/// threshold would otherwise drop despite corroborating token evidence
/// ([FUSION-SHARED-SUBTREE]). Only those pairs are measured: aligning
/// two subtrees for all candidates would repeat the admission-cost
/// mistake [FUSION-CONTENT-GATE] deliberately avoids, and a pair that
/// already survives needs no rescue.
pub fn apply_shared_subtree_rescue(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) {
    let mut measurer = OverlapMeasurer::new(trees);
    let mut tally = RescueTally::new();
    for pair in pairs.iter_mut() {
        tally.scan();
        measure_one(pair, fingerprints, &mut measurer, &mut tally);
    }
    tally.report_total(measurer.stats());
}

/// Measures one pair when it is eligible, resolvable, and cross-file,
/// recording every gate it passes.
fn measure_one(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    if !rescue_eligible(pair) {
        return;
    }
    tally.eligible();
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return;
    };
    if !crosses_files(left, right) {
        return;
    }
    tally.cross_file();
    pair.shared_subtree_overlap = measurer.rescue_overlap(left, right);
    tally.measure(
        pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP,
        measurer.stats(),
    );
}

/// True when the pair's endpoints live in different files.
///
/// The rescue is deliberately cross-file only. Every clone this route
/// exists to recover is a copy *between* files ([FUSION-SHARED-SUBTREE],
/// gh #408), and admitting same-file pairs on shape overlap is the
/// #197 in-file sibling-family shape, which the report already spends a
/// dedicated proof suppressing. It is also what keeps a single-file
/// corpus intact: same-file rescues union that file's subtrees into one
/// transitive component, and the same-file overlap collapse then
/// reduces it to a single logical location, which is dropped below
/// `MIN_REPORTABLE_MEMBERS` — so the file's real duplication
/// disappeared entirely rather than being reported
/// (`issue_119_role_gate_exercised`).
fn crosses_files(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.file_id != right.file_id
}

/// True for a pair worth measuring: dropped below its fused floor on a
/// zero structural anchor, yet carrying the token corroboration and
/// endpoint substance the rescue route requires.
fn rescue_eligible(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    score.structural <= 0.0
        && score.bounded_fused() < pair.fused_min_score
        && score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
}
