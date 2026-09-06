//! Unit tests for [`super`] — the [PIPELINE-CLUSTER-ELECT] partition
//! that stops one token bridge from welding two structural families into
//! a component that reports neither.
//!
//! Members are identified by their subtree hash alone, so these build
//! fingerprints directly rather than parsing: the pass reads nothing
//! else, and a corpus would only obscure which input drives which
//! outcome.

use std::collections::HashMap;

use super::*;
use crate::{
    ast::ByteRange,
    pair::FusedEdge,
    state::{FileId, FileRegistry},
};

/// The one language every member is parsed from, unless a test says
/// otherwise. The pass reads it only to confirm the component does not
/// span grammars.
const LANGUAGE: &str = "csharp";

/// A second grammar, so a component can span languages.
const OTHER_LANGUAGE: &str = "python";

/// The summing loop's normalised subtree.
const SUM_HASH: u8 = 1;

/// The multiplying loop's normalised subtree — a different tree, which
/// no rename and no literal edit can turn into [`SUM_HASH`].
const PRODUCT_HASH: u8 = 2;

/// A third distinct tree, so a component can hold three families.
const QUOTIENT_HASH: u8 = 3;

/// A fourth distinct tree, so two enclosing views can be digest
/// singletons — the two API classes of `rank_structural_only_policy`,
/// which hold different method counts and so never share a hash.
const REMAINDER_HASH: u8 = 4;

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
                })
        })
        .collect();
    FusedCluster {
        members,
        edges,
        shape_family: None,
    }
}

/// The member index lists of `clusters`, for direct comparison.
fn member_lists(clusters: &[FusedCluster]) -> Vec<Vec<usize>> {
    clusters
        .iter()
        .map(|cluster| cluster.members.clone())
        .collect()
}

/// Builds one fingerprint per entry of `(hash, file, start, end)`, so a test
/// can place two families inside one file at overlapping ranges.
fn placed_corpus(members: &[(u8, usize, usize, usize)]) -> Vec<Fingerprint> {
    let mut registry = FileRegistry::new();
    let files: Vec<FileId> = (0..members.len())
        .map(|position| registry.register(format!("case{position}.cs").into()))
        .collect();
    let fallback = registry.register("unplaced.cs".into());
    members
        .iter()
        .map(|(tag, file, start, end)| {
            let mut hash = [0_u8; 32];
            if let Some(first) = hash.first_mut() {
                *first = *tag;
            }
            Fingerprint {
                hash,
                file_id: *files.get(*file).unwrap_or(&fallback),
                byte_range: ByteRange {
                    start: *start,
                    end: *end,
                },
                node_count: NODE_COUNT,
            }
        })
        .collect()
}

/// Every file behind `fingerprints`, all parsed from [`LANGUAGE`].
fn one_language(fingerprints: &[Fingerprint]) -> HashMap<FileId, &'static str> {
    fingerprints
        .iter()
        .map(|member| (member.file_id, LANGUAGE))
        .collect()
}

/// Splits `fused_clusters` over a corpus that speaks a single language.
fn elect(fused_clusters: Vec<FusedCluster>, fingerprints: &[Fingerprint]) -> Vec<FusedCluster> {
    split_structural_families(fused_clusters, fingerprints, &one_language(fingerprints))
}

/// Splits one fully connected component over `hashes`.
fn split(hashes: &[u8]) -> Vec<FusedCluster> {
    let fingerprints = corpus(hashes);
    elect(vec![component(hashes.len())], &fingerprints)
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
        shape_family: None,
    };

    assert_eq!(
        member_lists(&elect(
            vec![untouched, component(fingerprints.len())],
            &fingerprints
        )),
        vec![vec![0, 1], vec![0, 1], vec![2, 3]],
        "the pass maps over the whole batch and keeps input order, so a \
         component needing no split is neither reordered nor lost"
    );
}

// [PIPELINE-CLUSTER-SUBSUME] owns the nesting case, and this pass must not
// pre-empt it. A copied method and the statement run inside it are two
// structural families over the *same* bytes; publishing both would show one
// duplicate as two findings and count its lines twice.
#[test]
fn families_covering_the_same_bytes_at_different_depths_are_left_whole() {
    // Two files, each holding an enclosing view (0..200) and the nested run
    // inside it (40..120).
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 0, 200),
        (SUM_HASH, 1, 0, 200),
        (PRODUCT_HASH, 0, 40, 120),
        (PRODUCT_HASH, 1, 40, 120),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(4)], &fingerprints)),
        vec![vec![0, 1, 2, 3]],
        "the nested run lies inside the enclosing view, so these are one \
         duplication seen at two depths — subsumption elects between them, \
         and splitting here would publish both"
    );
}

#[test]
fn families_in_one_file_that_do_not_touch_are_still_split() {
    // One file per family pair, ranges that share no byte.
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 0, 100),
        (SUM_HASH, 1, 0, 100),
        (PRODUCT_HASH, 0, 200, 300),
        (PRODUCT_HASH, 1, 200, 300),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(4)], &fingerprints)),
        vec![vec![0, 1], vec![2, 3]],
        "sharing a file is not sharing bytes; two disjoint runs welded by a \
         token edge are still two clusters"
    );
}

// `csharp-mcp` in full: two two-file clones that share no byte, plus a
// shallow shape duplicated across all four files that encloses each of them
// in half the corpus. The bridge covers code neither clone reaches, so it is
// a view of neither and must not glue them together.
#[test]
fn a_bridge_family_enclosing_two_disjoint_clones_does_not_merge_them() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 0, 200),
        (SUM_HASH, 1, 0, 200),
        (PRODUCT_HASH, 2, 0, 200),
        (PRODUCT_HASH, 3, 0, 200),
        (QUOTIENT_HASH, 0, 50, 100),
        (QUOTIENT_HASH, 1, 50, 100),
        (QUOTIENT_HASH, 2, 50, 100),
        (QUOTIENT_HASH, 3, 50, 100),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(8)], &fingerprints)),
        vec![vec![0, 1], vec![2, 3], vec![4, 5, 6, 7]],
        "one-way enclosure is not a nesting: the summing clone, the \
         multiplying clone and the four-file shape are three findings, and \
         reporting their union reports none of them"
    );
}

// #112 in miniature, and the reason a region count is not a weld. An
// enclosing view seen in two files, and the run nested inside it that also
// appears in a third: mutual coverage fails one way, so these are two
// regions — but every region touches every other, so no token edge glued
// anything and there is nothing to undo. Splitting published the enclosing
// pair as a cluster of its own, narrow enough to slip under the spread
// floors that were hiding the pattern.
#[test]
fn an_enclosing_view_reaching_fewer_files_than_its_nested_run_is_not_a_weld() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 0, 200),
        (SUM_HASH, 1, 0, 200),
        (PRODUCT_HASH, 0, 50, 200),
        (PRODUCT_HASH, 1, 50, 200),
        (PRODUCT_HASH, 2, 50, 200),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(5)], &fingerprints)),
        vec![vec![0, 1, 2, 3, 4]],
        "the enclosing pair and the nested run overlap in both files they \
         share, so the component holds one piece of code at two depths; \
         publishing the pair alone reports a narrower view of a family the \
         spread floors were hiding"
    );
}

// The weld the pass exists for survives the check: the two clones share no
// byte with each other, so a disjoint pair of regions is present even
// though the bridge touches both.
#[test]
fn a_bridge_touching_both_clones_does_not_hide_the_weld_between_them() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 0, 200),
        (SUM_HASH, 1, 0, 200),
        (PRODUCT_HASH, 2, 0, 200),
        (PRODUCT_HASH, 3, 0, 200),
        (QUOTIENT_HASH, 0, 50, 100),
        (QUOTIENT_HASH, 1, 50, 100),
        (QUOTIENT_HASH, 2, 50, 100),
        (QUOTIENT_HASH, 3, 50, 100),
    ]);
    let elected = elect(vec![component(8)], &fingerprints);

    assert_eq!(
        member_lists(&elected),
        vec![vec![0, 1], vec![2, 3], vec![4, 5, 6, 7]],
        "the summing clone and the multiplying clone share no byte, so the \
         weld is present and the bridge that touches both must not conceal it"
    );
    assert_eq!(
        elected.len(),
        3,
        "the bridge is a finding too — a shape duplicated across four \
         files — and dropping it would trade one false negative for another"
    );
}

// [CONFIG-CROSS-LANGUAGE]: a port of one algorithm into another language is
// a different normalised subtree by construction, so the digest says
// nothing about whether it is a copy. Splitting on it deletes the very
// finding the opt-in exists to produce.
#[test]
fn a_component_spanning_two_languages_is_left_whole() {
    let fingerprints = corpus(&[SUM_HASH, SUM_HASH, PRODUCT_HASH, PRODUCT_HASH]);
    let languages: HashMap<FileId, &'static str> = fingerprints
        .iter()
        .enumerate()
        .map(|(position, member)| {
            let language = if position < MIN_FAMILY_MEMBERS {
                LANGUAGE
            } else {
                OTHER_LANGUAGE
            };
            (member.file_id, language)
        })
        .collect();

    assert_eq!(
        member_lists(&split_structural_families(
            vec![component(fingerprints.len())],
            &fingerprints,
            &languages,
        )),
        vec![vec![0, 1, 2, 3]],
        "two grammars produce two digests whatever the code says, so the \
         cross-language cluster the user opted into must survive intact"
    );
}

// The guard refuses what it cannot confirm. A member whose file carries no
// language is not evidence that the component speaks one, and splitting on
// an unknown grammar is the cross-language false negative by another route.
#[test]
fn a_component_with_an_unresolved_language_is_left_whole() {
    let fingerprints = corpus(&[SUM_HASH, SUM_HASH, PRODUCT_HASH, PRODUCT_HASH]);
    let languages: HashMap<FileId, &'static str> = fingerprints
        .iter()
        .take(MIN_FAMILY_MEMBERS)
        .map(|member| (member.file_id, LANGUAGE))
        .collect();

    assert_eq!(
        member_lists(&split_structural_families(
            vec![component(fingerprints.len())],
            &fingerprints,
            &languages,
        )),
        vec![vec![0, 1, 2, 3]],
        "two of the four members have no language on record, so the \
         component cannot be shown to speak one and must not be split"
    );
}

mod container_tests;
