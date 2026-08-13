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

use std::collections::HashMap;

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster},
    fingerprint::Fingerprint,
    lsh::Signature,
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
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let members = vec![
        member(alpha, left[0], 1),
        member(beta, left[1], 1),
        member(alpha, right[0], 2),
        member(beta, right[1], 2),
    ];
    let signatures: Vec<Signature> = members.iter().map(|_| [11_u64; 128]).collect();
    let fused = [
        FusedCluster {
            members: vec![0, 1],
        },
        FusedCluster {
            members: vec![2, 3],
        },
    ];
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();
    build_ranked_fused_clusters(&members, &signatures, &vectors, &fused)
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
        let expected: Vec<Vec<(usize, usize)>> = case
            .expected
            .iter()
            .map(|span| vec![*span, *span])
            .collect();
        assert_eq!(spans(&clusters), expected, "{}", case.why);
    }
}
