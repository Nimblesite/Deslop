//! [PIPELINE-CLUSTER-SUBSUME] The region predicate: the two shapes that
//! collapse, and every shape that must not.

use super::{published, spans};

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
