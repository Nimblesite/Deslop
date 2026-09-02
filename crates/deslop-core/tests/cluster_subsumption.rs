//! Cross-cluster subsumption boundary ([PIPELINE-CLUSTER-SUBSUME]).
//!
//! Subsumption deletes a whole cluster from the report, so the predicate
//! that decides "these two are one duplication" is a false-negative
//! surface: every byte it is too generous about is a finding the user
//! never sees. The two shapes it must accept are strict enclosure (a
//! duplicated method and the statement clones nested inside it) and the
//! *crossed* case (two views of one whole-file duplicate whose depth
//! difference falls on opposite sides in each file, so neither occurrence
//! set nests inside the other).
//!
//! Everything else it must refuse. Two clusters that merely intersect —
//! partially, one-sidedly, or by a single byte where one ends and the
//! next begins — describe two regions that happen to touch, and both are
//! findings. This suite pins both directions: the shapes that must
//! collapse, and the shapes that must not.

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs},
    fingerprint::Fingerprint,
    pair::FusedCluster,
    state::{FileId, FileRegistry},
};

/// A cluster member over `[start, end)` of `file_id`, digest `digest`.
fn member(file_id: FileId, span: (usize, usize), digest: u8) -> Fingerprint {
    Fingerprint {
        hash: [digest; 32],
        file_id,
        byte_range: ByteRange {
            start: span.0,
            end: span.1,
        },
        node_count: 40,
    }
}

/// Builds two same-shape clusters over two files and returns whatever
/// survives ranking and subsumption.
///
/// `left` and `right` each give the byte span used in *both* files, so
/// every cluster names the same two files. That isolates the region
/// predicate: the file-coverage guard cannot be what decides the
/// outcome, because neither view names a file the other omits.
fn published(left: [(usize, usize); 2], right: [(usize, usize); 2]) -> Vec<Cluster> {
    published_views(&[left, right])
}

/// [`published`] for any number of same-shape views over the same two
/// files; view `n` carries digest `n + 1`.
fn published_views(views: &[[(usize, usize); 2]]) -> Vec<Cluster> {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let members: Vec<Fingerprint> = views
        .iter()
        .zip(1u8..)
        .flat_map(|(view, digest)| {
            [
                member(alpha, view[0], digest),
                member(beta, view[1], digest),
            ]
        })
        .collect();
    let fused: Vec<FusedCluster> = (0..members.len())
        .step_by(2)
        .map(|left| FusedCluster {
            members: vec![left, left.saturating_add(1)],
            edges: Vec::new(),
            shape_family: None,
        })
        .collect();
    build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: &members,
        fused_clusters: &fused,
        trees: &[],
        file_languages: &std::collections::HashMap::new(),
        file_paths: &std::collections::HashMap::new(),
    })
}

/// The published clusters' occurrence spans, in rank order.
fn spans(clusters: &[Cluster]) -> Vec<Vec<(usize, usize)>> {
    clusters
        .iter()
        .map(|cluster| {
            cluster
                .members
                .iter()
                .map(|found| (found.byte_range.start, found.byte_range.end))
                .collect()
        })
        .collect()
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
