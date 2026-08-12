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

/// [PIPELINE-CLUSTER-SUBSUME] Partial overlap is two findings. Neither
/// view contains the other in either file, so neither re-describes the
/// other: they are two duplicated regions that happen to share bytes,
/// and deleting one erases a duplicate the report never mentions again.
#[test]
fn two_partially_overlapping_regions_are_both_published() {
    let clusters = published([(0, 100), (0, 100)], [(50, 150), (50, 150)]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(0, 100), (0, 100)], vec![(50, 150), (50, 150)]],
        "half-overlapping regions are two duplicates, not one described twice"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] One shared byte is not region equivalence.
/// Adjacent duplicated regions that touch where one ends and the next
/// begins are the cheapest way to lose a finding, because a single
/// intersecting byte is indistinguishable from a full re-description to
/// a predicate built on intersection.
#[test]
fn regions_sharing_a_single_byte_are_both_published() {
    let clusters = published([(0, 100), (0, 100)], [(99, 200), (99, 200)]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(99, 200), (99, 200)], vec![(0, 100), (0, 100)]],
        "one shared byte does not make two regions one duplication"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] A small region that merely reaches into a
/// wider one is still its own finding. The overlap is one-sided — the
/// wide view extends far past the small one and the small one starts
/// before the wide one — so neither is a re-description of the other.
#[test]
fn a_region_overhanging_a_wider_one_survives_it() {
    let clusters = published([(0, 100), (0, 100)], [(95, 500), (95, 500)]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(95, 500), (95, 500)], vec![(0, 100), (0, 100)]],
        "an overhanging region is not contained, so it is not re-described"
    );
}

/// Control: disjoint regions were never at risk, and stay published.
/// Without this, a subsumption rule that deleted everything would still
/// satisfy the collapse assertions above.
#[test]
fn disjoint_regions_are_both_published() {
    let clusters = published([(0, 100), (0, 100)], [(200, 300), (200, 300)]);
    assert_eq!(
        spans(&clusters),
        vec![vec![(0, 100), (0, 100)], vec![(200, 300), (200, 300)]],
        "regions that share no bytes are unrelated findings"
    );
}
