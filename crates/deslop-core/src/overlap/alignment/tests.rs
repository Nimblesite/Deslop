//! [FUSION-SHARED-SUBTREE] The alignment's arithmetic, pinned against an
//! independent reference.
//!
//! `tree_edit_distance` is a keyroot-decomposed dynamic program with a
//! reused scratch grid and interned kinds — fast, and completely opaque
//! to inspection. Every `structural` value in every report is
//! `max(nodes) - TED`, so an error here changes bucket routing, ranking,
//! the duplication metric and cross-cluster subsumption while every
//! other test still passes.
//!
//! So the distance is asserted against [`reference_distance`]: the
//! textbook recursive forest recurrence, written on the tree structure
//! itself with no post-order indexing, no keyroots and no grid. It
//! shares no code with the implementation, so the two agreeing on a
//! deterministic corpus of shapes is real evidence rather than one
//! algorithm confirming itself.

use super::{Aligner, PostNode};
use crate::overlap::shapes::{forest_nodes, generate, postorder, Lcg, RefTree, SEED};

/// Trees compared per generated case: every pair drawn from this many
/// shapes, so the corpus is quadratic in it.
const GENERATED_SHAPES: usize = 60;

/// Largest generated tree, in nodes. The reference recurrence is
/// exponential without memoisation, which is exactly what keeps it
/// obviously correct; this bound is what keeps it fast.
const MAX_GENERATED_NODES: usize = 9;

/// Nodes in the wide-tree regression fixture.
const WIDE_TREE_NODES: usize = 128;

/// Every node differs, so each exact relabel contributes one edit.
const EXPECTED_WIDE_TREE_DISTANCE: usize = WIDE_TREE_NODES;

/// Singleton rows and columns are exact without forest DP, leaving root/root.
const EXPECTED_WIDE_TREE_FOREST_RUNS: usize = 1;

/// Distinct fixture kinds make every aligned node exercise relabelling.
const WIDE_LEFT_KIND: &str = "alpha";

/// Distinct fixture kinds make every aligned node exercise relabelling.
const WIDE_RIGHT_KIND: &str = "beta";

/// Third kind used to pin matching and non-matching singleton paths.
const OTHER_KIND: &str = "gamma";

/// Flat tree postorder: leaves followed by their common root.
fn wide_sequence(kind: &'static str) -> Vec<PostNode> {
    let mut nodes = (1..WIDE_TREE_NODES)
        .map(|position| PostNode {
            kind,
            leftmost: position,
        })
        .collect::<Vec<_>>();
    nodes.push(PostNode { kind, leftmost: 1 });
    nodes
}

/// [PERF-FLUTTER-TODO-RESCUE] One-node keyroot pairs have an exact
/// constant-time answer and must not enter the general forest DP. Wide ASTs
/// contain quadratically many such pairs; routing them through the grid was
/// the dominant stack in the ten-second Flutter-rescue profile.
#[test]
fn singleton_keyroot_pairs_skip_full_forest_dp() {
    let left = wide_sequence(WIDE_LEFT_KIND);
    let right = wide_sequence(WIDE_RIGHT_KIND);
    let mut aligner = Aligner::default();

    let distance = aligner.distance(&left, &right);

    assert_eq!(
        distance, EXPECTED_WIDE_TREE_DISTANCE,
        "the shortcut must preserve the exact edit distance"
    );
    assert_eq!(
        aligner.forest_runs, EXPECTED_WIDE_TREE_FOREST_RUNS,
        "leaf pairs have an exact 0/1 answer and must not run the full forest DP"
    );
}

/// [PERF-FLUTTER-TODO-RESCUE] Singleton-to-tree shortcuts must preserve the
/// textbook distance whether the retained node is the root, is nested away
/// from the leftmost spine, or has no matching kind. Both directions are
/// asserted because they write different matrix strides.
#[test]
fn singleton_tree_shortcuts_match_the_textbook_in_both_directions() {
    let cases = [
        RefTree {
            kind: WIDE_LEFT_KIND,
            children: vec![RefTree {
                kind: OTHER_KIND,
                children: Vec::new(),
            }],
        },
        RefTree {
            kind: OTHER_KIND,
            children: vec![RefTree {
                kind: WIDE_RIGHT_KIND,
                children: vec![RefTree {
                    kind: WIDE_LEFT_KIND,
                    children: Vec::new(),
                }],
            }],
        },
        RefTree {
            kind: OTHER_KIND,
            children: vec![RefTree {
                kind: WIDE_RIGHT_KIND,
                children: Vec::new(),
            }],
        },
    ];
    let singleton = RefTree {
        kind: WIDE_LEFT_KIND,
        children: Vec::new(),
    };
    for (index, tree) in cases.iter().enumerate() {
        assert_shortcut_matches_reference(index, &singleton, tree);
        assert_shortcut_matches_reference(index, tree, &singleton);
    }
}

/// Compares one shortcut direction with the independent forest recurrence.
fn assert_shortcut_matches_reference(index: usize, left: &RefTree, right: &RefTree) {
    let expected = reference_distance(std::slice::from_ref(left), std::slice::from_ref(right));
    let left_sequence = postorder(left);
    let right_sequence = postorder(right);
    let mut aligner = Aligner::default();
    let measured = aligner.distance(&left_sequence, &right_sequence);
    assert_eq!(
        measured, expected,
        "singleton case {index} must retain its exact distance"
    );
    assert_eq!(
        aligner.forest_runs, 0,
        "singleton case {index} must bypass forest DP"
    );
}

/// Ordered tree edit distance between two forests, by the textbook
/// recurrence: delete the rightmost root, insert the rightmost root, or
/// match the two rightmost roots and recurse into both their children
/// and the forests to their left. Unit insert/delete/relabel costs.
fn reference_distance(left: &[RefTree], right: &[RefTree]) -> usize {
    let (Some((left_last, left_rest)), Some((right_last, right_rest))) =
        (left.split_last(), right.split_last())
    else {
        return forest_nodes(left).saturating_add(forest_nodes(right));
    };
    let deleted = promote(left_rest, &left_last.children);
    let inserted = promote(right_rest, &right_last.children);
    let relabel = usize::from(left_last.kind != right_last.kind);
    reference_distance(&deleted, right)
        .saturating_add(1)
        .min(reference_distance(left, &inserted).saturating_add(1))
        .min(
            reference_distance(&left_last.children, &right_last.children)
                .saturating_add(reference_distance(left_rest, right_rest))
                .saturating_add(relabel),
        )
}

/// The forest left by deleting a rightmost root: the trees to its left,
/// followed by its own children promoted in its place.
fn promote(rest: &[RefTree], children: &[RefTree]) -> Vec<RefTree> {
    let mut promoted = rest.to_vec();
    promoted.extend_from_slice(children);
    promoted
}

/// [FUSION-SHARED-SUBTREE] The keyroot-decomposed dynamic program must
/// return exactly the textbook recurrence's distance, on every shape.
///
/// The corpus is every ordered pair drawn from [`GENERATED_SHAPES`]
/// deterministically generated trees, so it covers equal trees,
/// relabels, insertions at every depth, and wholly unrelated shapes.
/// Both directions of each pair are asserted: the distance is
/// symmetric, and a decomposition bug is often not.
#[test]
fn the_dynamic_program_matches_the_textbook_recurrence() {
    let mut source = Lcg(SEED);
    let shapes: Vec<RefTree> = (0..GENERATED_SHAPES)
        .map(|_| generate(&mut source, MAX_GENERATED_NODES))
        .collect();
    let sequences: Vec<Vec<PostNode>> = shapes.iter().map(postorder).collect();
    let mut compared = 0_usize;
    let mut non_trivial = 0_usize;
    for (left_index, left) in shapes.iter().enumerate() {
        for (right_index, right) in shapes.iter().enumerate() {
            let expected =
                reference_distance(std::slice::from_ref(left), std::slice::from_ref(right));
            let (Some(left_sequence), Some(right_sequence)) =
                (sequences.get(left_index), sequences.get(right_index))
            else {
                continue;
            };
            let measured = Aligner::default().distance(left_sequence, right_sequence);
            assert_eq!(
                measured, expected,
                "shape {left_index} against shape {right_index}: the dynamic program \
                 measured {measured} edits, the textbook recurrence {expected}. Left \
                 postorder {left_sequence:?}, right {right_sequence:?}"
            );
            compared = compared.saturating_add(1);
            if expected > 0 {
                non_trivial = non_trivial.saturating_add(1);
            }
        }
    }
    let expected_comparisons = GENERATED_SHAPES.saturating_mul(GENERATED_SHAPES);
    assert_eq!(
        compared, expected_comparisons,
        "every generated pair must be compared, or this test proves less than it claims"
    );
    // A corpus that happened to generate one shape repeatedly would
    // agree trivially at distance zero and assert nothing.
    assert!(
        non_trivial > expected_comparisons / 2,
        "over half the generated pairs must differ, or the agreement is vacuous — only \
         {non_trivial} of {expected_comparisons} had a non-zero distance"
    );
}

/// [FUSION-SHARED-SUBTREE] A distance is zero exactly when the two
/// sequences are the same shape with the same kinds. Pinned separately
/// because the reference agreeing on generated shapes cannot show that
/// *identity* is the zero case rather than some other coincidence.
#[test]
fn identical_shapes_cost_nothing_and_differing_ones_cost_something() {
    let mut source = Lcg(SEED);
    let shapes: Vec<RefTree> = (0..GENERATED_SHAPES)
        .map(|_| generate(&mut source, MAX_GENERATED_NODES))
        .collect();
    for (index, shape) in shapes.iter().enumerate() {
        let sequence = postorder(shape);
        let distance = Aligner::default().distance(&sequence, &sequence);
        assert_eq!(
            distance, 0,
            "shape {index} against itself must cost nothing, cost {distance}"
        );
        let relabelled = postorder(&RefTree {
            kind: "delta",
            children: shape.children.clone(),
        });
        let relabel_cost = Aligner::default().distance(&sequence, &relabelled);
        assert_eq!(
            relabel_cost, 1,
            "shape {index} with only its root renamed must cost exactly one relabel, \
             cost {relabel_cost}"
        );
    }
}

/// [PERF-FLUTTER-TODO-RESCUE] One [`Aligner`] reused across a run of
/// pairs must measure exactly what a fresh one measures for each pair.
///
/// This is the contract the whole reuse optimisation rests on and the
/// only one the reference comparison above cannot see: it builds a new
/// aligner per pair, so a grid cell left behind by the *previous*
/// alignment and spliced into the next would pass it untouched. The
/// pairs are deliberately run in a mixed order of sizes, because a
/// stale value only survives into a later pair whose grid is smaller
/// than the one that wrote it.
#[test]
fn a_reused_aligner_measures_what_a_fresh_one_does() {
    let mut source = Lcg(SEED);
    let shapes: Vec<RefTree> = (0..GENERATED_SHAPES)
        .map(|_| generate(&mut source, MAX_GENERATED_NODES))
        .collect();
    let mut sequences: Vec<Vec<PostNode>> = shapes.iter().map(postorder).collect();
    // Largest first, so every later pair reuses grids sized for a
    // bigger problem — the arrangement a stale cell survives.
    sequences.sort_by_key(|sequence| std::cmp::Reverse(sequence.len()));
    let mut reused = Aligner::default();
    let mut checked = 0_usize;
    let mut widths = std::collections::BTreeSet::new();
    for (left_index, left) in sequences.iter().enumerate() {
        for right in sequences.iter().skip(left_index) {
            let fresh = Aligner::default().distance(left, right);
            let carried = reused.distance(left, right);
            assert_eq!(
                carried, fresh,
                "a reused aligner measured {carried} edits where a fresh one measured \
                 {fresh}; left postorder {left:?}, right {right:?}"
            );
            checked = checked.saturating_add(1);
            let _inserted = widths.insert(right.len());
        }
    }
    assert!(
        checked > GENERATED_SHAPES,
        "the reuse must be exercised over many pairs, ran only {checked}"
    );
    assert!(
        widths.len() > 1,
        "every pair was the same width, so no grid was ever reused at a smaller size \
         and this proves nothing — saw widths {widths:?}"
    );
}
