//! [FUSED-CANDIDATE-BUCKET-RECOVERY] The walk asks the weld, emits every
//! weld, skips existing candidates, and never leaves a copy behind.

use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::{
    ast::ByteRange,
    pair::{FusedEdge, PairScore},
    state::FileRegistry,
};

const COMPLETE_PAIR: &[usize] = &[0, 1];
const COMPLETE_PAIR_EDGES: &[(usize, usize)] = &[(0, 1)];
const OPEN_CHAIN: &[usize] = &[2, 3, 4];
const OPEN_CHAIN_EDGES: &[(usize, usize)] = &[(2, 3), (3, 4)];
const COMPLETE_TRIANGLE: &[usize] = &[5, 6, 7];
const COMPLETE_TRIANGLE_EDGES: &[(usize, usize)] = &[(5, 6), (5, 7), (6, 7)];
const REPEATED_EDGE: &[(usize, usize)] = &[(2, 3), (2, 3), (3, 4)];
/// Each open chain welds its first member to both others; with no
/// existing candidates both welds are new pairs.
const OPEN_CHAIN_RECOVERED: &[(usize, usize)] = &[(2, 3), (2, 4)];
/// Two incomplete families, two welds each.
const EXPECTED_PROBES: usize = 4;
const EXPECTED_WORKLOAD: (usize, usize, usize) = (2, 6, 3);
const FIXTURE_FINGERPRINTS: usize = 8;
// One mixed shape family holds two repeatable hashes and one singleton,
// but only equal hashes can recover.
const HASH_FIRST: [u8; 32] = [1; 32];
const HASH_SECOND: [u8; 32] = [2; 32];
const HASH_SINGLETON: [u8; 32] = [3; 32];
const MIXED_HASHES: [[u8; 32]; 5] = [
    HASH_FIRST,
    HASH_FIRST,
    HASH_SECOND,
    HASH_SECOND,
    HASH_SINGLETON,
];
const MIXED_MEMBERS: &[usize] = &[0, 1, 2, 3, 4];
const MIXED_EDGES: &[(usize, usize)] = &[(0, 2), (2, 3), (3, 4)];
const MIXED_BRIDGE_EDGES: &[(usize, usize)] = &[(0, 2), (1, 2), (2, 4), (3, 4)];
const FIRST_HASH_PAIR: (usize, usize) = (0, 1);
const SECOND_HASH_PAIR: (usize, usize) = (2, 3);
const EXPECTED_HASH_GROUPS: usize = 2;
const FIXTURE_PATH: &str = "mixed.rs";
const FIXTURE_START: usize = 0;
const FIXTURE_END: usize = 1;
const FIXTURE_NODES: usize = 40;
const NONTRANSITIVE_HASHES: [[u8; 32]; 5] =
    [HASH_SECOND, HASH_FIRST, HASH_FIRST, HASH_FIRST, HASH_FIRST];
const NONTRANSITIVE_MEMBERS: &[usize] = &[0, 1, 2, 3, 4];
const NONTRANSITIVE_EDGES: &[(usize, usize)] = &[(0, 1), (0, 2), (1, 3), (2, 4)];
const CLONE_RELATIONS: &[(usize, usize)] = &[(0, 1), (0, 2), (1, 3), (2, 4), (3, 4)];
const OLD_RECOVERABLE_EDGE: (usize, usize) = (3, 4);
const NONTRANSITIVE_EXISTING: &[(usize, usize)] = &[(1, 3), (2, 4)];
const NO_SCORE: f64 = 0.0;
const EXACT_SCORE: f64 = 1.0;
const ONE_NEW_PAIR: usize = 1;
/// A same-hash bucket large enough that pair-by-pair certification would be
/// unmistakable: 400 members are 79,800 pairs.
const COPY_BUCKET_MEMBERS: usize = 400;
/// The bucket's second member, the first whose pairs are not already the star.
const COPY_BUCKET_SECOND: usize = 1;
/// The star centre's edges already exist and the second member starts the
/// component, so neither costs a verdict.
const COPY_BUCKET_UNJUDGED: usize = 2;
/// A family of one hash: the star centre and three copies behind it.
const SINGLE_HASH_MEMBERS: &[usize] = &[0, 1, 2, 3];
/// The star every member of that family already has.
const SINGLE_HASH_STAR: &[(usize, usize)] = &[(0, 1), (0, 2), (0, 3)];
/// The pairs that keep the copies together when the gate drops the
/// star centre: each later copy welds to the first copy behind it.
const SINGLE_HASH_RECOVERED: &[(usize, usize)] = &[(1, 2), (1, 3)];

fn fixture_fingerprints(hashes: &[[u8; 32]]) -> Vec<Fingerprint> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from(FIXTURE_PATH));
    hashes
        .iter()
        .map(|hash| Fingerprint {
            hash: *hash,
            file_id,
            byte_range: ByteRange {
                start: FIXTURE_START,
                end: FIXTURE_END,
            },
            node_count: FIXTURE_NODES,
        })
        .collect()
}

fn family(members: &[usize], edges: &[(usize, usize)]) -> FusedCluster {
    FusedCluster {
        members: members.to_vec(),
        edges: edges
            .iter()
            .map(|&(left, right)| FusedEdge { left, right })
            .collect(),
        shape_family: None,
    }
}

fn candidate((left, right): (usize, usize)) -> CandidatePair {
    CandidatePair {
        left,
        right,
        endpoint_node_counts: (FIXTURE_NODES, FIXTURE_NODES),
        lsh_only_node_floor: FIXTURE_NODES,
        lsh_only_min_jaccard: NO_SCORE,
        fused_min_score: crate::pair::FUSED_THRESHOLD,
        shared_subtree_overlap: NO_SCORE,
        verified_async_core: false,
        score: PairScore {
            structural: EXACT_SCORE,
            token_jaccard: NO_SCORE,
            embedding_cos: NO_SCORE,
        },
    }
}

fn keys(pairs: &[CandidatePair]) -> Vec<(usize, usize)> {
    pairs.iter().map(|pair| (pair.left, pair.right)).collect()
}

/// A weld that joins every pair and counts how often it was asked.
fn welding_everything(
    probes: &AtomicUsize,
) -> impl FnMut(usize, usize) -> Option<CandidatePair> + '_ {
    move |left, right| {
        let _previous = probes.fetch_add(1, Ordering::Relaxed);
        Some(candidate((left, right)))
    }
}

/// A weld that refuses to be asked about two different hashes.
fn welding_equal_hashes<'a>(
    fingerprints: &'a [Fingerprint],
    probes: &'a AtomicUsize,
) -> impl FnMut(usize, usize) -> Option<CandidatePair> + 'a {
    move |left, right| {
        let hashes = fingerprints
            .get(left)
            .zip(fingerprints.get(right))
            .map(|(left, right)| (left.hash, right.hash));
        assert!(
            hashes.is_some_and(|(left, right)| left == right),
            "copy recovery probed a cross-hash pair it cannot recover"
        );
        let _previous = probes.fetch_add(1, Ordering::Relaxed);
        Some(candidate((left, right)))
    }
}

/// A weld that joins exactly the recorded relations.
fn welding_relations(left: usize, right: usize) -> Option<CandidatePair> {
    CLONE_RELATIONS
        .contains(&(left.min(right), left.max(right)))
        .then(|| candidate((left, right)))
}

fn recovered<W: CopyWeld + Send>(
    families: &[FusedCluster],
    fingerprints: &[Fingerprint],
    open: &(impl Fn() -> W + Sync),
    existing: &[CandidatePair],
) -> Vec<CandidatePair> {
    recovered_pairs(families, fingerprints, open, existing, &AtomicUsize::new(0))
}

// A complete shape family cannot contain a missing copy edge, while a
// chain or repeated edge can hide one.
#[test]
fn recovery_walks_only_families_with_a_missing_pair() {
    let fingerprints = fixture_fingerprints(&[HASH_FIRST; FIXTURE_FINGERPRINTS]);
    let families = [
        family(COMPLETE_PAIR, COMPLETE_PAIR_EDGES),
        family(OPEN_CHAIN, OPEN_CHAIN_EDGES),
        family(COMPLETE_TRIANGLE, COMPLETE_TRIANGLE_EDGES),
        family(OPEN_CHAIN, REPEATED_EDGE),
    ];
    let probes = AtomicUsize::new(0);
    assert_eq!(
        recovery_workload(&families, &fingerprints),
        EXPECTED_WORKLOAD
    );
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_everything(&probes),
        &[],
    );
    let expected: Vec<(usize, usize)> = OPEN_CHAIN_RECOVERED
        .iter()
        .chain(OPEN_CHAIN_RECOVERED)
        .copied()
        .collect();
    assert_eq!(keys(&pairs), expected);
    assert_eq!(probes.load(Ordering::Relaxed), EXPECTED_PROBES);
}

#[test]
fn recovery_probes_only_equal_hash_members_of_a_mixed_shape_family() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_EDGES)];
    let probes = AtomicUsize::new(0);
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_equal_hashes(&fingerprints, &probes),
        &[],
    );
    assert_eq!(keys(&pairs), vec![FIRST_HASH_PAIR, SECOND_HASH_PAIR]);
    assert_eq!(
        recovery_workload(&families, &fingerprints),
        (
            EXPECTED_HASH_GROUPS,
            EXPECTED_HASH_GROUPS,
            EXPECTED_HASH_GROUPS
        )
    );
    assert_eq!(probes.load(Ordering::Relaxed), EXPECTED_HASH_GROUPS);
}

// A mixed family holding one bucket of copies behind an unrelated star
// centre is recovered in about one verdict per member, not one per pair
// (gh #506).
#[test]
fn recovery_walks_a_copied_bucket_in_linear_verdicts() {
    let mut hashes = vec![HASH_FIRST; COPY_BUCKET_MEMBERS];
    hashes.push(HASH_SECOND);
    let fingerprints = fixture_fingerprints(&hashes);
    let members: Vec<usize> = (0..hashes.len()).collect();
    let star: Vec<(usize, usize)> = (1..hashes.len()).map(|member| (0, member)).collect();
    let families = [family(&members, &star)];
    let existing: Vec<_> = star.iter().copied().map(candidate).collect();
    let probes = AtomicUsize::new(0);
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_equal_hashes(&fingerprints, &probes),
        &existing,
    );
    let expected: Vec<(usize, usize)> = (2..COPY_BUCKET_MEMBERS)
        .map(|member| (COPY_BUCKET_SECOND, member))
        .collect();
    assert_eq!(
        keys(&pairs),
        expected,
        "every copy joins through the first member whose star edge is not already a candidate"
    );
    assert_eq!(
        probes.load(Ordering::Relaxed),
        COPY_BUCKET_MEMBERS - COPY_BUCKET_UNJUDGED,
        "a bucket of copies is walked, not certified pair by pair"
    );
}

// [FUSED-CANDIDATE-BUCKET-STAR] A family of one hash is walked like any
// other: the copies behind the star centre weld to each other, so the
// gate dropping the centre's edges cannot pull them apart. The star
// itself is never rebuilt.
#[test]
fn recovery_welds_the_copies_behind_a_single_hash_star_centre() {
    let fingerprints = fixture_fingerprints(&[HASH_FIRST; SINGLE_HASH_MEMBERS.len()]);
    let families = [family(SINGLE_HASH_MEMBERS, SINGLE_HASH_STAR)];
    let existing: Vec<_> = SINGLE_HASH_STAR.iter().copied().map(candidate).collect();
    let probes = AtomicUsize::new(0);
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_everything(&probes),
        &existing,
    );
    assert_eq!(keys(&pairs), SINGLE_HASH_RECOVERED.to_vec());
    assert_eq!(probes.load(Ordering::Relaxed), SINGLE_HASH_RECOVERED.len());
}

// A pre-existing shape-family edge cannot produce a missing structural
// pair, so it needs no new verdict.
#[test]
fn recovery_skips_an_existing_equal_hash_edge() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_EDGES)];
    let existing = [candidate(SECOND_HASH_PAIR)];
    let probes = AtomicUsize::new(0);
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_equal_hashes(&fingerprints, &probes),
        &existing,
    );
    assert_eq!(keys(&pairs), vec![FIRST_HASH_PAIR]);
    assert_eq!(probes.load(Ordering::Relaxed), ONE_NEW_PAIR);
}

// A retained candidate can be absent from the pre-gate family edges yet
// still cannot be rebuilt.
#[test]
fn recovery_skips_a_candidate_absent_from_family_edges() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_BRIDGE_EDGES)];
    let mut pending = candidate(SECOND_HASH_PAIR);
    pending.score.structural = NO_SCORE;
    pending.score.token_jaccard = crate::pair::SHARED_SUBTREE_MIN_JACCARD;
    let probes = AtomicUsize::new(0);
    let pairs = recovered(
        &families,
        &fingerprints,
        &|| welding_equal_hashes(&fingerprints, &probes),
        &[pending],
    );
    assert_eq!(keys(&pairs), vec![FIRST_HASH_PAIR]);
    assert_eq!(probes.load(Ordering::Relaxed), ONE_NEW_PAIR);
}

// Skipping known edges must keep a third same-hash clone even when clone
// relations are nontransitive.
#[test]
fn recovery_keeps_a_missing_nontransitive_copy() {
    let fingerprints = fixture_fingerprints(&NONTRANSITIVE_HASHES);
    let families = [family(NONTRANSITIVE_MEMBERS, NONTRANSITIVE_EDGES)];
    let existing: Vec<_> = NONTRANSITIVE_EXISTING
        .iter()
        .copied()
        .map(candidate)
        .collect();
    let pairs = recovered(&families, &fingerprints, &|| welding_relations, &existing);
    let recovered_edges: BTreeSet<(usize, usize)> = keys(&pairs).into_iter().collect();
    assert!(recovered_edges.contains(&OLD_RECOVERABLE_EDGE));
    assert!(NONTRANSITIVE_EXISTING
        .iter()
        .all(|edge| !recovered_edges.contains(edge)));
}

/// The same-hash edges a greedy grouping would have built as stars.
fn greedy_edges(groups: &[Vec<usize>], fingerprints: &[Fingerprint]) -> BTreeSet<(usize, usize)> {
    groups
        .iter()
        .filter_map(|members| members.split_first())
        .flat_map(|(reference, rest)| {
            rest.iter().filter_map(move |member| {
                let left = fingerprints.get(*reference)?;
                let right = fingerprints.get(*member)?;
                (left.hash == right.hash)
                    .then_some(((*reference).min(*member), (*reference).max(*member)))
            })
        })
        .collect()
}

#[test]
fn the_walk_connects_everything_the_greedy_grouping_connected() {
    let fingerprints = fixture_fingerprints(&NONTRANSITIVE_HASHES);
    let families = [family(NONTRANSITIVE_MEMBERS, NONTRANSITIVE_EDGES)];
    let greedy: Vec<Vec<usize>> =
        crate::buckets::grouping::group_members(NONTRANSITIVE_MEMBERS, |left, right| {
            welding_relations(left, right).map(|_| crate::buckets::ClusterKind::NearlyIdentical)
        })
        .into_iter()
        .map(|(members, _kind)| members)
        .collect();
    let pairs = recovered(&families, &fingerprints, &|| welding_relations, &[]);
    let old_edges = greedy_edges(&greedy, &fingerprints);
    let new_edges: BTreeSet<(usize, usize)> = keys(&pairs).into_iter().collect();
    assert!(old_edges.contains(&OLD_RECOVERABLE_EDGE));
    assert!(
        new_edges.is_superset(&old_edges),
        "the walk lost a same-hash copy the greedy grouping found: {old_edges:?} vs {new_edges:?}"
    );
}
