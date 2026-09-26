//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Two views that overhang one
//! nested view on different sides are padded readings of it.

use deslop_core::{cluster::Cluster, state::FileRegistry};

use super::{
    in_rank_order, occurrences, published_views, published_weighted, ranked_with_sources, spans,
    HEAVY_NODES, LIGHTEST_NODES, LIGHT_NODES, MEMBER_NODES,
};

/// Source length and copied byte for the two unmergeable exact windows.
const COPIED_SOURCE_LEN: usize = 250;
const COPIED_BYTE: u8 = b'x';
const COPIED_MEMBER_COUNT: usize = 2;
const COPIED_PATHS: [&str; COPIED_MEMBER_COUNT] = ["alpha.ts", "beta.ts"];
const LEFT_WINDOW_RANGE: (usize, usize) = (0, 200);
const RIGHT_WINDOW_RANGE: (usize, usize) = (50, COPIED_SOURCE_LEN);
const COPIED_CORE_RANGE: (usize, usize) = (50, 200);

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] If source bytes prove the union is
/// copied but there is no AST sibling run to join, neither window is padding.
fn copied_windows() -> Vec<Cluster> {
    let mut registry = FileRegistry::new();
    let alpha = registry.register(COPIED_PATHS[0].into());
    let beta = registry.register(COPIED_PATHS[1].into());
    let pair = |span: (usize, usize)| vec![(alpha, span), (beta, span)];
    let views = [
        (MEMBER_NODES, pair(LEFT_WINDOW_RANGE)),
        (MEMBER_NODES, pair(RIGHT_WINDOW_RANGE)),
        (LIGHT_NODES, pair(COPIED_CORE_RANGE)),
    ];
    let sources = [
        (alpha, vec![COPIED_BYTE; COPIED_SOURCE_LEN]),
        (beta, vec![COPIED_BYTE; COPIED_SOURCE_LEN]),
    ]
    .into();
    ranked_with_sources(&views, &sources)
}

#[test]
fn copied_straddling_windows_are_not_discarded_without_a_join() {
    let clusters = copied_windows();
    let mut actual = spans(&clusters);
    actual.sort_unstable();
    assert_eq!(
        actual,
        vec![
            vec![LEFT_WINDOW_RANGE; COPIED_MEMBER_COUNT],
            vec![RIGHT_WINDOW_RANGE; COPIED_MEMBER_COUNT]
        ]
    );
}

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Two windows that overhang one
/// nested view on different sides are padded readings of it: the nested
/// view is the finding, and the padding it never shared goes with the
/// windows. The straddlers are ranked first (more mass), so the nested
/// view is absorbed by the first of them before the straddle is met and
/// must come back when both die.
#[test]
fn two_windows_straddling_one_nested_view_publish_that_view() {
    let left_padded = [(0, 200), (0, 200)];
    let right_padded = [(50, 250), (50, 250)];
    let nested = [(50, 200), (50, 200)];
    let clusters = published_views(&[left_padded, right_padded, nested]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(50, 200), (50, 200)]],
        "the view both straddlers contain is the one finding"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] A view that yielded to a
/// straddler comes back when the straddler dies. The decoy `(0, 100)`
/// out-masses every other view, so it is scanned first and yields to the
/// left-padded view that encloses it; that view then dies with its
/// straddling twin in favour of the core `(50, 200)`. The core does not
/// reach bytes 0..50, so unless the decoy is released the duplication it
/// reports leaves the report: an absorbed view is released when its
/// absorber is removed ([PIPELINE-CLUSTER-SUBSUME]), and the straddle
/// releases whatever the dropped views absorbed.
#[test]
fn a_view_that_yielded_to_a_straddler_is_released_when_it_dies() {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let pair = |span: (usize, usize)| vec![(alpha, span), (beta, span)];
    let decoy = (HEAVY_NODES, pair((0, 100)));
    let left_padded = (MEMBER_NODES, pair((0, 200)));
    let right_padded = (LIGHT_NODES, pair((50, 250)));
    let core = (LIGHTEST_NODES, pair((50, 200)));
    let clusters = published_weighted(&[decoy, left_padded, right_padded, core]);
    assert!(in_rank_order(&clusters), "survivors stay in rank order");
    assert_eq!(
        occurrences(&clusters),
        vec![pair((0, 100)), pair((50, 200))],
        "the decoy yielded to a straddler and comes back when the \
         straddler dies; the core is the straddle's finding"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] A view nested in one straddler
/// only is not what the two share, so the overlap stays two findings.
#[test]
fn a_view_nested_in_only_one_straddler_leaves_both_published() {
    let left_padded = [(0, 200), (0, 200)];
    let right_padded = [(50, 250), (50, 250)];
    let nested_in_left_only = [(10, 40), (10, 40)];
    let clusters = published_views(&[left_padded, right_padded, nested_in_left_only]);
    let mut actual = spans(&clusters);
    actual.sort_unstable();
    assert_eq!(
        actual,
        vec![vec![(0, 200), (0, 200)], vec![(50, 250), (50, 250)]],
        "without a view inside both, the straddlers are two findings and \
         the left-only nested view collapses into its encloser"
    );
}
