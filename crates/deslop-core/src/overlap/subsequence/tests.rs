//! [FUSED-SHARED-SUBTREE-BOUND-ORDER] The ordered bound, pinned two
//! ways: it must equal the textbook longest-common-subsequence dynamic
//! program, and it must never fall below the shared node mass the
//! alignment it stands in for actually measures.
//!
//! The second assertion is the accuracy one. The rescue skips the
//! alignment whenever this bound lands under the admission floor, so a
//! bound that ever *under*-states the achievable overlap silently drops
//! a real cross-file duplicate — a false negative no other test in the
//! tree would notice. It is asserted over every ordered pair of a
//! deterministic shape corpus, in both directions.

use std::collections::HashMap;

use super::{common_subsequence_len, KindPositions, Row};
use crate::overlap::{
    alignment::{Aligner, PostNode},
    shapes::{generate, postorder, Lcg, RefTree, KINDS, SEED},
};

/// Shapes generated for the soundness corpus; every ordered pair is
/// asserted, so the case count is quadratic in this.
const GENERATED_SHAPES: usize = 60;

/// Largest generated tree, in nodes. Well past a machine word so the
/// bit-parallel row's carries cross word boundaries under assertion.
const MAX_GENERATED_NODES: usize = 90;

/// Flat-sequence lengths asserted against the reference program. Chosen
/// around the 64-bit word boundary, where an off-by-one in the carry
/// chain or the live-bit mask would otherwise hide.
const WORD_EDGE_LENGTHS: [usize; 9] = [0, 1, 63, 64, 65, 127, 128, 129, 200];

/// The synthetic root [`crate::overlap::view::build_view`] appends;
/// present here so the asserted sequences are the ones production
/// aligns.
const WINDOW_ROOT: &str = "__window__";

/// Textbook longest common subsequence over two kind sequences: the
/// full O(n·m) table, written on the sequences themselves with no bit
/// tricks, no masks and no carries. Shares no code with the
/// implementation.
fn reference_subsequence(left: &[&'static str], right: &[&'static str]) -> usize {
    let mut previous = vec![0_usize; right.len().saturating_add(1)];
    let mut current = previous.clone();
    for left_kind in left {
        for (index, right_kind) in right.iter().enumerate() {
            let taken = previous.get(index).copied().unwrap_or(0).saturating_add(1);
            let skipped = previous
                .get(index.saturating_add(1))
                .copied()
                .unwrap_or(0)
                .max(current.get(index).copied().unwrap_or(0));
            let best = if left_kind == right_kind {
                taken
            } else {
                skipped
            };
            if let Some(slot) = current.get_mut(index.saturating_add(1)) {
                *slot = best;
            }
        }
        previous.clone_from(&current);
    }
    previous.last().copied().unwrap_or(0)
}

/// The measured bound for two kind sequences.
fn measured_subsequence(left: &[&'static str], right: &[&'static str]) -> usize {
    let left_nodes = flat_nodes(left);
    let right_nodes = flat_nodes(right);
    let positions = KindPositions::new(&right_nodes, right.len());
    let mut row = Row::default();
    common_subsequence_len(&left_nodes, left.len(), &positions, &mut row)
}

/// A flat post-order run of leaves carrying `kinds`.
fn flat_nodes(kinds: &[&'static str]) -> Vec<PostNode> {
    kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| PostNode {
            kind,
            leftmost: index.saturating_add(1),
        })
        .collect()
}

/// A deterministic kind sequence of `length` kinds.
fn sequence(source: &mut Lcg, length: usize) -> Vec<&'static str> {
    (0..length)
        .map(|_| KINDS.get(source.below(KINDS.len())).copied().unwrap_or(""))
        .collect()
}

/// The kind multiset intersection — the [FUSED-SHARED-SUBTREE-BOUND]
/// bound, recomputed here independently so the ordered bound can be
/// shown to be no looser than it.
fn multiset_shared(left: &[&'static str], right: &[&'static str]) -> usize {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for kind in right {
        let count = counts.entry(kind).or_insert(0);
        *count = count.saturating_add(1);
    }
    left.iter()
        .filter(|kind| {
            counts
                .get_mut(*kind)
                .filter(|count| **count > 0)
                .map(|count| *count = count.saturating_sub(1))
                .is_some()
        })
        .count()
}

/// One tree's production-shaped post-order: its own nodes, then the
/// synthetic window root the view appends.
fn windowed(tree: &RefTree) -> (Vec<PostNode>, usize) {
    let mut nodes = postorder(tree);
    let total = nodes.len();
    nodes.push(PostNode {
        kind: WINDOW_ROOT,
        leftmost: 1,
    });
    (nodes, total)
}

/// The deterministic shape corpus, largest-first so the widest cases
/// are asserted even if the generator draws small.
fn corpus() -> Vec<RefTree> {
    let mut source = Lcg(SEED);
    (0..GENERATED_SHAPES)
        .map(|_| generate(&mut source, MAX_GENERATED_NODES))
        .collect()
}

/// [FUSED-SHARED-SUBTREE-BOUND-ORDER] The bit-parallel row must return
/// exactly the textbook table's answer, at and around every word
/// boundary, in both argument orders — the subsequence length is
/// symmetric and a carry-chain bug usually is not.
#[test]
fn the_bit_parallel_row_matches_the_textbook_table() {
    let mut source = Lcg(SEED);
    let mut asserted = 0_usize;
    let mut non_trivial = 0_usize;
    for left_length in WORD_EDGE_LENGTHS {
        for right_length in WORD_EDGE_LENGTHS {
            let left = sequence(&mut source, left_length);
            let right = sequence(&mut source, right_length);
            let expected = reference_subsequence(&left, &right);
            assert_eq!(
                measured_subsequence(&left, &right),
                expected,
                "{left_length} kinds against {right_length}: the bit-parallel row disagreed with the textbook table",
            );
            assert_eq!(
                measured_subsequence(&right, &left),
                expected,
                "{right_length} kinds against {left_length}: the bound is not symmetric",
            );
            assert!(
                expected <= left_length.min(right_length),
                "a common subsequence of {left_length} and {right_length} kinds cannot be {expected} long",
            );
            asserted += 1;
            if expected > 0 {
                non_trivial += 1;
            }
        }
    }
    assert_eq!(
        asserted,
        WORD_EDGE_LENGTHS.len() * WORD_EDGE_LENGTHS.len(),
        "every length pairing must be asserted",
    );
    assert!(
        non_trivial >= asserted / 2,
        "only {non_trivial} of {asserted} cases shared any kinds at all — the corpus asserts nothing",
    );
}

/// [FUSED-SHARED-SUBTREE-BOUND-ORDER] The accuracy assertion: the
/// bound may never state less than the alignment achieves. Every
/// ordered pair of the shape corpus is measured both ways — the exact
/// Zhang–Shasha shared mass the rescue would have computed, and the
/// bound the rescue substitutes for it — and the bound must dominate.
#[test]
fn the_bound_never_understates_what_the_alignment_measures() {
    let shapes = corpus();
    let mut aligner = Aligner::default();
    let mut row = Row::default();
    let mut asserted = 0_usize;
    let mut non_trivial = 0_usize;
    for left_tree in &shapes {
        let (left_nodes, left_total) = windowed(left_tree);
        for right_tree in &shapes {
            let (right_nodes, right_total) = windowed(right_tree);
            let distance = aligner.distance(&left_nodes, &right_nodes);
            let shared = left_total.max(right_total).saturating_sub(distance);
            let positions = KindPositions::new(&right_nodes, right_total);
            let bound = common_subsequence_len(&left_nodes, left_total, &positions, &mut row);
            assert!(
                bound >= shared,
                "a {left_total}-node shape against a {right_total}-node shape: \
                 the alignment shares {shared} nodes but the bound allowed only {bound}",
            );
            asserted += 1;
            if shared > 0 {
                non_trivial += 1;
            }
        }
    }
    assert_eq!(
        asserted,
        GENERATED_SHAPES * GENERATED_SHAPES,
        "every ordered pair of the corpus must be asserted",
    );
    assert!(
        non_trivial > asserted / 2,
        "only {non_trivial} of {asserted} pairs shared any nodes — the corpus proves nothing",
    );
}

/// [FUSED-SHARED-SUBTREE-BOUND-ORDER] The bound is never looser than
/// the multiset bound it joins, and on scrambled order it is strictly
/// tighter — which is the whole reason it exists. Two endpoints holding
/// the same kinds in reversed order share every kind by multiset and
/// can align almost none of them.
#[test]
fn scrambled_order_is_bounded_far_below_the_shared_multiset() {
    let mut source = Lcg(SEED);
    let forward = sequence(&mut source, WORD_EDGE_LENGTHS[8]);
    let mut reversed = forward.clone();
    reversed.reverse();
    let multiset = multiset_shared(&forward, &reversed);
    let ordered = measured_subsequence(&forward, &reversed);
    assert_eq!(
        multiset,
        forward.len(),
        "a reversal shares every kind by multiset, so the multiset bound learns nothing",
    );
    assert!(
        ordered <= multiset,
        "the ordered bound ({ordered}) must never exceed the multiset bound ({multiset})",
    );
    assert!(
        ordered < multiset,
        "the ordered bound must be strictly tighter than {multiset} on a reversed sequence, got {ordered}",
    );
    assert_eq!(
        ordered,
        reference_subsequence(&forward, &reversed),
        "the tighter value must still be the true longest common subsequence",
    );
    let identical = measured_subsequence(&forward, &forward);
    assert_eq!(
        identical,
        forward.len(),
        "identical sequences must bound at their full length, not below it",
    );
}
