//! [PIPELINE-CLUSTER-SUBSUME] The region predicate: the two shapes that
//! collapse, and every shape that must not.

use super::{published, published_across, spans, Cluster, View};
use deslop_core::state::FileRegistry;

const REPEATED_FILE: &str = "circuit.cs";
const BROAD_PAIR: [(usize, usize); 2] = [(0, 100), (120, 300)];
const THREE_COPIES: [(usize, usize); 3] = [(10, 50), (130, 170), (230, 270)];
const SPLIT_PAIR: [(usize, usize); 3] = [(0, 100), (150, 180), (200, 230)];
const MERGED_PAIR: [(usize, usize); 3] = [(10, 40), (50, 90), (140, 240)];
const NESTING_PAIR: [(usize, usize); 2] = [(0, 100), (120, 220)];
const REVERSED_NESTED_PAIR: [(usize, usize); 2] = [(130, 170), (10, 50)];
const DISTINCT_VIEWS: usize = 2;
const WIDE_FILES: [&str; 2] = ["alpha.rs", "beta.rs"];
const WIDE_SPAN: (usize, usize) = (0, 100);
const TWO_FUNCTIONS: [(usize, usize); 2] = [(10, 40), (50, 90)];

fn published_same_file(first: &[(usize, usize)], second: &[(usize, usize)]) -> Vec<Cluster> {
    let mut registry = FileRegistry::new();
    let file = registry.register(REPEATED_FILE.into());
    let views = [first, second].map(|spans| spans.iter().map(|span| (file, *span)).collect());
    published_across(&views)
}

/// [PIPELINE-CLUSTER-SUBSUME] One broad occurrence cannot stand in for two copies.
#[test]
fn one_enclosing_window_does_not_erase_two_distinct_copies() {
    let clusters = published_same_file(&BROAD_PAIR, &THREE_COPIES);
    assert_eq!(clusters.len(), DISTINCT_VIEWS);
    assert_eq!(
        spans(&clusters),
        vec![THREE_COPIES.to_vec(), BROAD_PAIR.to_vec()]
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] Equal counts still need a one-to-one pairing.
#[test]
fn equal_counts_do_not_hide_a_split_and_merge() {
    let clusters = published_same_file(&SPLIT_PAIR, &MERGED_PAIR);
    assert_eq!(clusters.len(), DISTINCT_VIEWS);
    assert_eq!(
        spans(&clusters),
        vec![MERGED_PAIR.to_vec(), SPLIT_PAIR.to_vec()]
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] A window that holds the same two copies in
/// every file is that duplication read at a coarser grain (gh #232).
#[test]
fn a_window_holding_two_copies_in_every_file_absorbs_them() {
    let mut registry = FileRegistry::new();
    let files = WIDE_FILES.map(|name| registry.register(name.into()));
    let wide: View = files.iter().map(|file| (*file, WIDE_SPAN)).collect();
    let narrow: View = files
        .iter()
        .flat_map(|file| TWO_FUNCTIONS.map(|span| (*file, span)))
        .collect();
    let clusters = published_across(&[wide, narrow]);
    assert_eq!(clusters.len(), 1);
    assert_eq!(spans(&clusters), vec![vec![WIDE_SPAN; WIDE_FILES.len()]]);
}

/// [PIPELINE-CLUSTER-SUBSUME] Reordered nested copies are still one finding.
#[test]
fn distinct_nested_same_file_copies_collapse_in_position_order() {
    let clusters = published_same_file(&NESTING_PAIR, &REVERSED_NESTED_PAIR);
    assert_eq!(clusters.len(), 1);
    assert_eq!(spans(&clusters), vec![NESTING_PAIR.to_vec()]);
}

/// [PIPELINE-CLUSTER-SUBSUME] Strict enclosure collapses. The nested
/// view re-describes the enclosing duplication; publishing both shows
/// the same duplicate twice and double-counts it in the metrics.
#[test]
fn a_nested_view_collapses_into_the_view_that_encloses_it() {
    let clusters = published([(0, 200), (0, 200)], [(10, 50), (10, 50)]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(0, 200), (0, 200)]],
        "the enclosing 200-byte view is the duplication; the nested window \
         re-describes it"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] The crossed case collapses. Two views of
/// one whole-file duplicate can differ by a few bytes in opposite
/// directions per file, so neither occurrence set nests inside the
/// other — yet each occurrence still pairs by containment with one of
/// the other's, which is what makes them one duplication.
#[test]
fn two_crossed_views_of_one_whole_file_duplicate_collapse() {
    let clusters = published([(0, 238), (0, 234)], [(0, 237), (0, 235)]);
    assert_eq!(
        clusters.len(),
        1,
        "one whole-file duplicate described twice must publish once, got {:?}",
        spans(&clusters)
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] Every shape that must NOT collapse.
///
/// Each row is a distinct way two clusters can touch without either
/// re-describing the other, and each was a separate way to lose a
/// finding:
///
/// - **Partial overlap** — neither view contains the other in either
///   file; two duplicated regions that happen to share bytes.
/// - **A single shared byte** — where one region ends and the next
///   begins. The cheapest way to lose a finding, because one
///   intersecting byte is indistinguishable from a full re-description
///   to a predicate built on intersection.
/// - **A one-sided overhang** — the small region reaches into the wide
///   one but starts before it and the wide one extends far past it, so
///   the overlap is one-sided and neither is contained.
/// - **Disjoint regions** — the control. Without it, a subsumption rule
///   that deleted everything would still satisfy the collapse
///   assertions above.
///
/// Table-driven because the assertion is identical for every row: only
/// the spans and the expected publication order differ, and a row that
/// regressed would otherwise be a copy of its siblings.
/// One non-collapse row: why it must publish, the two spans, and the
/// order the pair must appear in.
struct TouchingCase {
    why: &'static str,
    first: (usize, usize),
    second: (usize, usize),
    expected: [(usize, usize); 2],
}

#[test]
fn regions_that_merely_touch_are_all_published() {
    let cases = [
        TouchingCase {
            why: "half-overlapping regions are two duplicates, not one described twice",
            first: (0, 100),
            second: (50, 150),
            expected: [(0, 100), (50, 150)],
        },
        TouchingCase {
            why: "one shared byte does not make two regions one duplication",
            first: (0, 100),
            second: (99, 200),
            expected: [(99, 200), (0, 100)],
        },
        TouchingCase {
            why: "an overhanging region is not contained, so it is not re-described",
            first: (0, 100),
            second: (95, 500),
            expected: [(95, 500), (0, 100)],
        },
        TouchingCase {
            why: "regions that share no bytes are unrelated findings",
            first: (0, 100),
            second: (200, 300),
            expected: [(0, 100), (200, 300)],
        },
    ];
    for case in cases {
        let clusters = published([case.first, case.first], [case.second, case.second]);
        let mut expected: Vec<Vec<(usize, usize)>> = case
            .expected
            .iter()
            .map(|span| vec![*span, *span])
            .collect();
        let mut actual = spans(&clusters);
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected, "{}", case.why);
        assert!(
            clusters.windows(2).all(|pair| match pair {
                [left, right] => left.id < right.id,
                _ => true,
            }),
            "equal-mass clusters sort by id after subsumption: {}",
            case.why
        );
    }
}
