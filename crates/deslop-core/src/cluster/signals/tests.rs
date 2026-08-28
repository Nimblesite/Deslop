//! [FUSION-CLUSTER-SIGNALS] Grouped signal measurement: the means are
//! the per-pair loop's exact values, and the cost is one valuation per
//! distinct group pair — the capture for the ranked build that spent
//! 90 of its 91 seconds re-deriving 3.6 million pair values on the
//! Flutter material corpus ([PERF-FLUTTER-TODO-PAIRS]).

use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    ast::{ByteRange, NormalizedNode},
    lsh::{SignatureIndex, SIGNATURE_LEN},
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
// same signature) plus one different structure. Three admitted pairs
// collapse onto two distinct group pairs, so the measurer answers
// exactly one hash-equal pair and runs exactly one alignment — while
// the rendered values are the strongest admitted pair's: structural
// max(1.0, v, v) = 1.0 and token max(1.0, j, j) = 1.0.
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
        file_tree(
            second,
            vec![branch("fn", ("call", "return"), second, 0, 40)],
        ),
    ];
    let fingerprints = vec![
        member(1, first, 0, 40),
        member(1, second, 0, 40),
        member(2, first, 50, 90),
    ];
    let signatures = vec![signature(7), signature(7), signature(9)];
    let signature_index = SignatureIndex::from_slice(&signatures);
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();

    let cross_overlap =
        OverlapMeasurer::new(&trees).overlap(&member(1, first, 0, 40), &member(2, first, 50, 90));
    let cross_jaccard = estimate_jaccard(&signature(7), &signature(9));
    assert!(
        cross_overlap > 0.0 && cross_overlap < 1.0,
        "fixture: the two structures must overlap partially so the \
         strongest-admitted-pair value actually exercises a measured \
         value, got {cross_overlap}"
    );
    assert!(
        (cross_jaccard - 0.0).abs() < f64::EPSILON,
        "fixture: disjoint constant signatures must estimate 0.0"
    );

    let mut measurer = OverlapMeasurer::new(&trees);
    let measured = measured_signals(
        &[0, 1, 2],
        &[(0, 1), (0, 2), (1, 2)],
        &fingerprints,
        &signature_index,
        &vectors,
        &mut measurer,
    );
    let triple = measured.score;

    assert_eq!(
        (
            triple.structural,
            triple.token_jaccard,
            triple.embedding_cos
        ),
        (1.0, 1.0, 0.0),
        "the rendered values are the strongest admitted pair's — the \
         hash-equal copy pair carries 1.0 on both axes"
    );
    assert_eq!(
        measured.source_pair,
        Some((0, 1)),
        "the hash-equal copy pair must be named as the signal source"
    );

    let stats = measurer.stats();
    assert_eq!(
        (stats.hash_equal, stats.alignments, stats.exact_hits),
        (1, 1, 0),
        "three admitted pairs of two structures must cost exactly one \
         hash-equal answer and one alignment — a count proportional to \
         the pair population means the group table failed, and a \
         corpus-scale cluster pays 3.6 million valuations again"
    );
}

// A member whose byte range resolves to no node may not share a group
// with the copies that resolve: an equal-hash pair is 1.0 either way,
// but its pairs against a *different* structure are 0.0, and one
// representative cannot stand for both answers. Measured pair by pair —
// each as the sole admitted pair — the unresolvable copy must report
// 0.0 against the different structure while the resolvable twin reports
// its measured overlap.
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
        file_tree(
            second,
            vec![branch("fn", ("call", "return"), second, 0, 40)],
        ),
    ];
    // The second copy's range matches no node in its tree.
    let fingerprints = vec![
        member(1, first, 0, 40),
        member(1, second, 3, 33),
        member(2, first, 50, 90),
    ];
    let signatures = vec![signature(7), signature(7), signature(9)];
    let signature_index = SignatureIndex::from_slice(&signatures);
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();

    let cross_overlap =
        OverlapMeasurer::new(&trees).overlap(&member(1, first, 0, 40), &member(2, first, 50, 90));

    // The resolvable copy against the different structure: the measured
    // overlap is published because the copy shares the resolvable group.
    let mut measurer = OverlapMeasurer::new(&trees);
    let resolvable = measured_signals(
        &[0, 1, 2],
        &[(0, 2)],
        &fingerprints,
        &signature_index,
        &vectors,
        &mut measurer,
    );
    assert_eq!(
        resolvable.score.structural.to_bits(),
        cross_overlap.to_bits(),
        "the resolvable twin must report the measured overlap against \
         the different structure"
    );

    // The unresolvable copy against the same different structure: 0.0,
    // because its range resolves to no node and it must not inherit the
    // twin's measured overlap through the group table.
    let mut measurer = OverlapMeasurer::new(&trees);
    let unresolvable = measured_signals(
        &[0, 1, 2],
        &[(1, 2)],
        &fingerprints,
        &signature_index,
        &vectors,
        &mut measurer,
    );
    assert_eq!(
        unresolvable.score.structural.to_bits(),
        0.0_f64.to_bits(),
        "an unresolvable copy shares the 1.0 of its equal-hash twin but \
         must not inherit the twin's measured overlap against the other \
         structure — folding it into the resolvable group would publish \
         {cross_overlap} for a pair the measurer answers 0.0"
    );
}

// [FUSION-CLUSTER-SIGNALS] gh #458 — a rendered cluster's signals are
// the strongest pair evidence that actually cleared admission, never a
// mean over pairs that did not. Two of the three pairs below are
// byte-identical (structural 1.0, token 1.0) yet are **not** part of the
// admitted pair set: the closure glued the component through the one
// measured pair, and the equal-hash combos are closure-only artifacts
// that never passed `admission.fused_threshold`. If the aggregation
// counts them, the rendered `structural` reads 1.0 — the byte proof of
// a pair the pipeline never admitted — and the denominator becomes
// `n*(n-1)/2` instead of the admitted-pair count.
#[test]
fn non_admitted_pairs_never_contribute_to_the_rendered_signals() {
    let mut registry = FileRegistry::new();
    let first = registry.register(PathBuf::from("admitted_a.rs"));
    let second = registry.register(PathBuf::from("admitted_b.rs"));

    let trees = vec![
        file_tree(
            first,
            vec![
                branch("fn", ("call", "return"), first, 0, 40),
                branch("fn", ("call", "field"), first, 50, 90),
            ],
        ),
        file_tree(
            second,
            vec![branch("fn", ("call", "return"), second, 0, 40)],
        ),
    ];
    let fingerprints = vec![
        member(1, first, 0, 40),
        member(2, first, 50, 90),
        member(1, second, 0, 40),
    ];
    let signatures = vec![signature(7), signature(9), signature(7)];
    let signature_index = SignatureIndex::from_slice(&signatures);
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();

    // (0,1) cleared admission — the pair whose measurement is the only
    // honest evidence the cluster carries. (0,2) and (1,2) never did,
    // even though (0,2) is byte-identical: the pair set is what matters.
    let cross_overlap =
        OverlapMeasurer::new(&trees).overlap(&member(1, first, 0, 40), &member(2, first, 50, 90));
    assert!(
        cross_overlap > 0.0 && cross_overlap < 1.0,
        "fixture: the admitted pair must measure a real partial overlap, \
         got {cross_overlap}"
    );

    let mut measurer = OverlapMeasurer::new(&trees);
    let measured = measured_signals(
        &[0, 1, 2],
        &[(0, 1)],
        &fingerprints,
        &signature_index,
        &vectors,
        &mut measurer,
    );
    let triple = measured.score;
    let mean_over_all_pairs = (cross_overlap + 1.0 + cross_overlap) / 3.0;

    assert_eq!(
        triple.structural.to_bits(),
        cross_overlap.to_bits(),
        "only the admitted pair (0,1) may contribute: the byte-identical \
         pair (0,2) never cleared admission, so its 1.0 must not leak \
         into the rendered structural — the mean over all three pairs \
         would publish {mean_over_all_pairs}"
    );
    assert_eq!(
        triple.token_jaccard.to_bits(),
        0.0_f64.to_bits(),
        "the admitted pair measures token 0.0; the closure-only pair \
         (0,2) measures 1.0 and must not appear in numerator or \
         denominator — the mean over all three pairs would publish \
         0.333..."
    );
    assert_eq!(
        triple.embedding_cos, 0.0,
        "embeddings off: no pair carries a vector"
    );
    assert_eq!(
        measured.source_pair,
        Some((0, 1)),
        "the admitted pair must be named as the signal source"
    );
}
