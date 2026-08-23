//! [FUSION-CLUSTER-SIGNALS] Grouped signal measurement: the means are
//! the per-pair loop's exact values, and the cost is one valuation per
//! distinct group pair — the capture for the ranked build that spent
//! 90 of its 91 seconds re-deriving 3.6 million pair values on the
//! Flutter material corpus ([PERF-FLUTTER-TODO-PAIRS]).

use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    ast::{ByteRange, NormalizedNode},
    lsh::SIGNATURE_LEN,
    state::{FileId, FileRegistry},
};

/// Two structurally identical subtrees and one that differs: the
/// smallest population where grouping shows. Kinds are grammar-like
/// names so an alignment has real relabel decisions to make.
fn leaf(kind: &'static str, file_id: FileId, start: usize, end: usize) -> NormalizedNode {
    NormalizedNode {
        kind,
        children: Vec::new(),
        byte_range: ByteRange { start, end },
        file_id,
    }
}

/// A two-leaf subtree whose kinds decide its structural identity.
fn branch(
    kind: &'static str,
    kinds: (&'static str, &'static str),
    file_id: FileId,
    start: usize,
    end: usize,
) -> NormalizedNode {
    let middle = start.saturating_add(end.saturating_sub(start) / 2);
    NormalizedNode {
        kind,
        children: vec![
            leaf(kinds.0, file_id, start, middle),
            leaf(kinds.1, file_id, middle, end),
        ],
        byte_range: ByteRange { start, end },
        file_id,
    }
}

/// One file holding the shapes at the given ranges.
fn file_tree(file_id: FileId, children: Vec<NormalizedNode>) -> NormalizedNode {
    let end = children
        .iter()
        .map(|child| child.byte_range.end)
        .max()
        .unwrap_or(0);
    NormalizedNode {
        kind: "module",
        children,
        byte_range: ByteRange { start: 0, end },
        file_id,
    }
}

/// A corpus-indexed fingerprint for an exact node.
fn member(hash_seed: u8, file_id: FileId, start: usize, end: usize) -> Fingerprint {
    Fingerprint {
        hash: [hash_seed; 32],
        file_id,
        byte_range: ByteRange { start, end },
        node_count: 3,
    }
}

/// A signature whose every slot is `fill`.
fn signature(fill: u64) -> Signature {
    [fill; SIGNATURE_LEN]
}

// Three occurrences: two copies of one structure (same Merkle hash,
// same signature) plus one different structure. Three pairs collapse
// onto two distinct group pairs, so the measurer answers exactly one
// hash-equal pair and runs exactly one alignment — while the means are
// the same three-summand means the per-pair loop produced, in the same
// order: ((1.0 + v) + v) / 3 for structural with v the measured
// cross-structure overlap, and ((1.0 + j) + j) / 3 for tokens.
#[test]
fn three_pairs_of_two_structures_cost_two_valuations() {
    let mut registry = FileRegistry::new();
    let first = registry.register(PathBuf::from("copies_a.rs"));
    let second = registry.register(PathBuf::from("copies_b.rs"));

    let trees = vec![
        file_tree(
            first,
            vec![
                branch("fn", ("call", "return"), first, 0, 40),
                branch("fn", ("call", "field"), first, 50, 90),
            ],
        ),
        file_tree(second, vec![branch("fn", ("call", "return"), second, 0, 40)]),
    ];
    let fingerprints = vec![
        member(1, first, 0, 40),
        member(1, second, 0, 40),
        member(2, first, 50, 90),
    ];
    let signatures = vec![signature(7), signature(7), signature(9)];
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();

    let cross_overlap =
        OverlapMeasurer::new(&trees).overlap(&member(1, first, 0, 40), &member(2, first, 50, 90));
    let cross_jaccard = estimate_jaccard(&signature(7), &signature(9));
    assert!(
        cross_overlap > 0.0 && cross_overlap < 1.0,
        "fixture: the two structures must overlap partially so the mean \
         actually exercises a measured value, got {cross_overlap}"
    );
    assert!(
        (cross_jaccard - 0.0).abs() < f64::EPSILON,
        "fixture: disjoint constant signatures must estimate 0.0"
    );

    let mut measurer = OverlapMeasurer::new(&trees);
    let triple = measured_signals(
        &[0, 1, 2],
        &fingerprints,
        &signatures,
        &vectors,
        &mut measurer,
    );

    let expected_structural = ((1.0 + cross_overlap) + cross_overlap) / 3.0;
    let expected_token = ((1.0 + cross_jaccard) + cross_jaccard) / 3.0;
    assert_eq!(
        (triple.structural, triple.token_jaccard, triple.embedding_cos),
        (expected_structural, expected_token, 0.0),
        "the grouped means must be bit-identical to the per-pair loop's \
         sums in the per-pair loop's order"
    );

    let stats = measurer.stats();
    assert_eq!(
        (stats.hash_equal, stats.alignments, stats.exact_hits),
        (1, 1, 0),
        "three pairs of two structures must cost exactly one hash-equal \
         answer and one alignment — a count proportional to the pair \
         population means the group table failed, and a corpus-scale \
         cluster pays 3.6 million valuations again"
    );
}

// A member whose byte range resolves to no node may not share a group
// with the copies that resolve: an equal-hash pair is 1.0 either way,
// but its pairs against a *different* structure are 0.0, and one
// representative cannot stand for both answers.
#[test]
fn an_unresolvable_copy_still_scores_its_unequal_pairs_zero() {
    let mut registry = FileRegistry::new();
    let first = registry.register(PathBuf::from("resolved.rs"));
    let second = registry.register(PathBuf::from("unresolved.rs"));

    let trees = vec![
        file_tree(
            first,
            vec![
                branch("fn", ("call", "return"), first, 0, 40),
                branch("fn", ("call", "field"), first, 50, 90),
            ],
        ),
        file_tree(second, vec![branch("fn", ("call", "return"), second, 0, 40)]),
    ];
    // The second copy's range matches no node in its tree.
    let fingerprints = vec![
        member(1, first, 0, 40),
        member(1, second, 3, 33),
        member(2, first, 50, 90),
    ];
    let signatures = vec![signature(7), signature(7), signature(9)];
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();

    let cross_overlap =
        OverlapMeasurer::new(&trees).overlap(&member(1, first, 0, 40), &member(2, first, 50, 90));

    let mut measurer = OverlapMeasurer::new(&trees);
    let triple = measured_signals(
        &[0, 1, 2],
        &fingerprints,
        &signatures,
        &vectors,
        &mut measurer,
    );

    // Pairs in loop order: (0,1) equal hash → 1.0; (0,2) measured;
    // (1,2) unresolvable against a different hash → 0.0.
    let expected_structural = ((1.0 + cross_overlap) + 0.0) / 3.0;
    assert_eq!(
        triple.structural.to_bits(),
        expected_structural.to_bits(),
        "an unresolvable copy shares the 1.0 of its equal-hash twin but \
         must not inherit the twin's measured overlap against the other \
         structure — folding it into the resolvable group would publish \
         {cross_overlap} for a pair the measurer answers 0.0"
    );
}
