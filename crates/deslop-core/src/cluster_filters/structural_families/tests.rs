//! Unit tests for [`super`] — the [PIPELINE-CLUSTER-ELECT] partition
//! that stops one token bridge from welding two structural families into
//! a component that reports neither.
//!
//! Members are identified by their subtree hash alone, so these build
//! fingerprints directly rather than parsing: the pass reads nothing
//! else, and a corpus would only obscure which input drives which
//! outcome.

use super::*;
use crate::{
    ast::ByteRange,
    pair::FusedEdge,
    state::{FileId, FileRegistry},
};

/// The summing loop's normalised subtree.
const SUM_HASH: u8 = 1;

/// The multiplying loop's normalised subtree — a different tree, which
/// no rename and no literal edit can turn into [`SUM_HASH`].
const PRODUCT_HASH: u8 = 2;

/// A third distinct tree, so a component can hold three families.
const QUOTIENT_HASH: u8 = 3;

/// Nodes per member. Well clear of any floor; the pass never reads it.
const NODE_COUNT: usize = 40;

/// Bytes per member, so every fingerprint spans a real range.
const MEMBER_BYTES: usize = 120;

/// Builds one fingerprint per entry of `hashes`, each in its own file,
/// so member index == position in `hashes`.
fn corpus(hashes: &[u8]) -> Vec<Fingerprint> {
    let mut registry = FileRegistry::new();
    hashes
        .iter()
        .enumerate()
        .map(|(position, tag)| {
            let file_id: FileId = registry.register(format!("case{position}.cs").into());
            let mut hash = [0_u8; 32];
            if let Some(first) = hash.first_mut() {
                *first = *tag;
            }
            Fingerprint {
                hash,
                file_id,
                byte_range: ByteRange {
                    start: 0,
                    end: MEMBER_BYTES,
                },
                node_count: NODE_COUNT,
            }
        })
        .collect()
}

/// One fully connected component over member indices `0..size`.
fn component(size: usize) -> FusedCluster {
    let members: Vec<usize> = (0..size).collect();
    let edges: Vec<FusedEdge> = members
        .iter()
        .flat_map(|left| {
            members
                .iter()
                .filter(move |right| *right > left)
                .map(move |right| FusedEdge {
                    left: *left,
                    right: *right,
                    strength: 1.0,
                })
        })
        .collect();
    FusedCluster { members, edges }
}

/// The member index lists of `clusters`, for direct comparison.
fn member_lists(clusters: &[FusedCluster]) -> Vec<Vec<usize>> {
    clusters
        .iter()
        .map(|cluster| cluster.members.clone())
        .collect()
}

/// Splits one fully connected component over `hashes`.
fn split(hashes: &[u8]) -> Vec<FusedCluster> {
    let fingerprints = corpus(hashes);
    split_structural_families(vec![component(hashes.len())], &fingerprints)
}

// The defect this module exists for: `csharp-mcp` in miniature — a
// summing pair and a multiplying pair welded into one component that
// reports neither.
#[test]
fn two_families_welded_into_one_component_are_reported_separately() {
    let elected = split(&[SUM_HASH, SUM_HASH, PRODUCT_HASH, PRODUCT_HASH]);

    assert_eq!(
        member_lists(&elected),
        vec![vec![0, 1], vec![2, 3]],
        "a summing pair and a multiplying pair are two clusters; \
         reporting their union reports neither"
    );
    for cluster in &elected {
        assert_eq!(
            cluster.edges.len(),
            1,
            "each family keeps only the edge whose endpoints both \
             stayed — an edge to a departed member is discovery \
             evidence for a pair that no longer exists: {cluster:?}"
        );
    }
}

#[test]
fn three_families_all_survive_the_split() {
    assert_eq!(
        member_lists(&split(&[
            SUM_HASH,
            PRODUCT_HASH,
            QUOTIENT_HASH,
            SUM_HASH,
            PRODUCT_HASH,
            QUOTIENT_HASH,
        ])),
        vec![vec![0, 3], vec![1, 4], vec![2, 5]],
        "families are emitted in first-member order and none is dropped"
    );
}

#[test]
fn a_family_with_a_near_miss_fringe_is_left_whole() {
    assert_eq!(
        member_lists(&split(&[SUM_HASH, SUM_HASH, SUM_HASH, PRODUCT_HASH])),
        vec![vec![0, 1, 2, 3]],
        "one structural family plus a lone near-miss is an ordinary \
         Type-3 cluster, and the near-miss is an occurrence a reader \
         wants — splitting here would delete it"
    );
}

#[test]
fn a_single_family_is_left_whole() {
    assert_eq!(
        member_lists(&split(&[SUM_HASH, SUM_HASH, SUM_HASH])),
        vec![vec![0, 1, 2]],
        "a three-way clone of one subtree is one cluster"
    );
}

#[test]
fn members_belonging_to_no_reportable_family_are_dropped() {
    assert_eq!(
        member_lists(&split(&[
            SUM_HASH,
            SUM_HASH,
            PRODUCT_HASH,
            PRODUCT_HASH,
            QUOTIENT_HASH,
        ])),
        vec![vec![0, 1], vec![2, 3]],
        "the lone third tree is an occurrence of nothing; publishing it \
         inside either family would report code that is not a copy"
    );
}

#[test]
fn a_component_of_strangers_is_left_whole() {
    assert_eq!(
        member_lists(&split(&[SUM_HASH, PRODUCT_HASH, QUOTIENT_HASH])),
        vec![vec![0, 1, 2]],
        "no member shares a subtree with another, so there is no family \
         to elect and nothing this pass can improve"
    );
}

#[test]
fn every_component_in_the_batch_is_considered() {
    let fingerprints = corpus(&[SUM_HASH, SUM_HASH, PRODUCT_HASH, PRODUCT_HASH]);
    let untouched = FusedCluster {
        members: vec![0, 1],
        edges: Vec::new(),
    };

    assert_eq!(
        member_lists(&split_structural_families(
            vec![untouched, component(fingerprints.len())],
            &fingerprints,
        )),
        vec![vec![0, 1], vec![0, 1], vec![2, 3]],
        "the pass maps over the whole batch and keeps input order, so a \
         component needing no split is neither reordered nor lost"
    );
}
