use std::{
    collections::{BTreeSet, HashMap},
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
const EXPECTED_GROUPS: usize = 2;
const EXPECTED_CALLS: usize = 2;
const EXPECTED_WORKLOAD: (usize, usize, usize) = (2, 6, 3);
const FIXTURE_FINGERPRINTS: usize = 8;
// [FUSED-CANDIDATE-BUCKET-RECOVERY] One mixed shape family holds two
// repeatable hashes and one singleton, but only equal hashes can recover.
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
const FIRST_GROUP: &[usize] = &[0, 1];
const SECOND_GROUP: &[usize] = &[2, 3];
const EXISTING_HASH_PAIR: (usize, usize) = (2, 3);
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
const INFO_SMALL_NODES: usize = 12;
const INFO_MIDDLE_NODES: usize = 39;
const INFO_NEAR_NODES: usize = 40;
const INFO_LARGE_NODES: usize = 100;
const INFO_NODE_COUNTS: [usize; 4] = [
    INFO_SMALL_NODES,
    INFO_MIDDLE_NODES,
    INFO_NEAR_NODES,
    INFO_LARGE_NODES,
];
const INFO_FAR_MEMBERS: &[usize] = &[0, 3];
const INFO_NEAR_MEMBERS: &[usize] = &[1, 2];
const INFO_MIXED_MEMBERS: &[usize] = &[0, 1, 2];
const INFO_MISSING_MEMBERS: &[usize] = &[0, INFO_NODE_COUNTS.len()];
const INFO_EQUAL_HASH_MEMBERS: &[usize] = &[0, 2];
const INFO_FAMILY_COUNT: usize = 3;
const INFO_ELIGIBLE_COUNT: usize = 2;
const HASH_FOURTH: [u8; 32] = [4; 32];

struct CountingJudge(AtomicUsize);

struct HashAwareJudge<'a> {
    fingerprints: &'a [Fingerprint],
    calls: AtomicUsize,
}

impl HashAwareJudge<'_> {
    fn measured(&self, members: &[usize]) -> ClusterKind {
        let hash = members
            .first()
            .and_then(|index| self.fingerprints.get(*index))
            .map(|fingerprint| fingerprint.hash);
        assert!(
            members.iter().all(|index| self
                .fingerprints
                .get(*index)
                .is_some_and(|fingerprint| Some(fingerprint.hash) == hash)),
            "copy recovery measured a cross-hash pair it cannot recover"
        );
        let _previous = self.calls.fetch_add(1, Ordering::Relaxed);
        ClusterKind::NearlyIdentical
    }
}

struct RelationJudge;

impl ClusterKindJudge for RelationJudge {
    fn kind(&self, members: &[usize]) -> Option<ClusterKind> {
        let [left, right] = members else {
            return None;
        };
        CLONE_RELATIONS
            .contains(&(*left.min(right), *left.max(right)))
            .then_some(ClusterKind::NearlyIdentical)
    }

    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        crate::buckets::grouping::group_members(members, |left, right| {
            CLONE_RELATIONS
                .contains(&(left.min(right), left.max(right)))
                .then_some(ClusterKind::NearlyIdentical)
        })
    }
}

fn recoverable_edges(
    groups: &[Vec<usize>],
    fingerprints: &[Fingerprint],
) -> BTreeSet<(usize, usize)> {
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

impl ClusterKindJudge for CountingJudge {
    fn kind(&self, _members: &[usize]) -> Option<ClusterKind> {
        None
    }

    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        let _previous = self.0.fetch_add(1, Ordering::Relaxed);
        vec![(members.to_vec(), ClusterKind::NearlyIdentical)]
    }
}

impl ClusterKindJudge for HashAwareJudge<'_> {
    fn kind(&self, members: &[usize]) -> Option<ClusterKind> {
        Some(self.measured(members))
    }

    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        vec![(members.to_vec(), self.measured(members))]
    }
}

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

// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] A far-only family cannot
// publish shape-only information; a mixed family keeps its near-sized pair.
#[test]
fn informational_count_bound_keeps_every_potential_shape_pair() {
    let mut fingerprints =
        fixture_fingerprints(&[HASH_FIRST, HASH_SECOND, HASH_SINGLETON, HASH_FOURTH]);
    for (found, count) in fingerprints.iter_mut().zip(INFO_NODE_COUNTS) {
        found.node_count = count;
    }
    let families = [
        family(INFO_FAR_MEMBERS, &[]),
        family(INFO_NEAR_MEMBERS, &[]),
        family(INFO_MIXED_MEMBERS, &[]),
    ];
    assert_eq!(families.len(), INFO_FAMILY_COUNT);
    let eligible = eligible_information(
        &families,
        &fingerprints,
        &[],
        &HashMap::new(),
        &HashMap::new(),
        crate::config::RoutingTuning::default().nearly_identical_min_shape,
    );
    assert_eq!(eligible.len(), INFO_ELIGIBLE_COUNT);
    let retained: Vec<_> = eligible
        .iter()
        .map(|found| found.members.as_slice())
        .collect();
    assert_eq!(retained, [INFO_NEAR_MEMBERS, INFO_MIXED_MEMBERS]);
    assert!(!eligible
        .iter()
        .any(|found| found.members == INFO_FAR_MEMBERS));
}

// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] An unresolved member or
// same-hash pair must remain available for the ordinary kind verdict.
#[test]
fn informational_count_bound_keeps_unresolved_and_equal_hash_pairs() {
    let mut fingerprints = fixture_fingerprints(&[HASH_FIRST, HASH_SECOND, HASH_FIRST]);
    for (found, count) in fingerprints.iter_mut().zip(INFO_NODE_COUNTS) {
        found.node_count = count;
    }
    let families = [
        family(INFO_MISSING_MEMBERS, &[]),
        family(INFO_EQUAL_HASH_MEMBERS, &[]),
    ];
    let eligible = eligible_information(
        &families,
        &fingerprints,
        &[],
        &HashMap::new(),
        &HashMap::new(),
        crate::config::RoutingTuning::default().nearly_identical_min_shape,
    );
    assert_eq!(eligible.len(), families.len());
    let retained: Vec<_> = eligible
        .iter()
        .map(|found| found.members.as_slice())
        .collect();
    assert_eq!(retained, [INFO_MISSING_MEMBERS, INFO_EQUAL_HASH_MEMBERS]);
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

fn existing_candidate((left, right): (usize, usize)) -> CandidatePair {
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

// [CLONE-KIND-FOLD] A complete shape family cannot contain a missing
// copy edge, while a chain or repeated edge can hide one.
#[test]
fn recovery_measures_only_families_with_a_missing_pair() {
    let fingerprints = fixture_fingerprints(&[HASH_FIRST; FIXTURE_FINGERPRINTS]);
    let families = [
        family(COMPLETE_PAIR, COMPLETE_PAIR_EDGES),
        family(OPEN_CHAIN, OPEN_CHAIN_EDGES),
        family(COMPLETE_TRIANGLE, COMPLETE_TRIANGLE_EDGES),
        family(OPEN_CHAIN, REPEATED_EDGE),
    ];
    let judge = CountingJudge(AtomicUsize::new(0));
    assert_eq!(
        recovery_workload(&families, &fingerprints),
        EXPECTED_WORKLOAD
    );
    let groups: Vec<_> = recovery_groups(&families, &fingerprints, &judge, &[]).collect();
    assert_eq!(groups.len(), EXPECTED_GROUPS);
    assert!(groups.iter().all(|members| members == OPEN_CHAIN));
    assert_eq!(judge.0.load(Ordering::Relaxed), EXPECTED_CALLS);
}

#[test]
fn recovery_judges_only_equal_hash_members_of_a_mixed_shape_family() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_EDGES)];
    let judge = HashAwareJudge {
        fingerprints: &fingerprints,
        calls: AtomicUsize::new(0),
    };
    let groups: Vec<_> = recovery_groups(&families, &fingerprints, &judge, &[]).collect();
    assert_eq!(groups, vec![FIRST_GROUP.to_vec(), SECOND_GROUP.to_vec()]);
    assert_eq!(
        recovery_workload(&families, &fingerprints),
        (
            EXPECTED_HASH_GROUPS,
            EXPECTED_HASH_GROUPS,
            EXPECTED_HASH_GROUPS
        )
    );
    assert_eq!(judge.calls.load(Ordering::Relaxed), EXPECTED_HASH_GROUPS);
}

// [FUSED-CANDIDATE-BUCKET-RECOVERY] A pre-existing shape-family edge
// cannot produce a missing structural pair, so it needs no new verdict.
#[test]
fn recovery_skips_an_existing_equal_hash_edge() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_EDGES)];
    let judge = HashAwareJudge {
        fingerprints: &fingerprints,
        calls: AtomicUsize::new(0),
    };
    let existing = [existing_candidate(EXISTING_HASH_PAIR)];
    let groups: Vec<_> = recovery_groups(&families, &fingerprints, &judge, &existing).collect();
    assert_eq!(groups, vec![FIRST_GROUP.to_vec()]);
    assert_eq!(judge.calls.load(Ordering::Relaxed), ONE_NEW_PAIR);
}

// [FUSED-CANDIDATE-BUCKET-RECOVERY] A retained candidate can be
// absent from the pre-gate family edges yet still cannot be rebuilt.
#[test]
fn recovery_skips_a_candidate_absent_from_family_edges() {
    let fingerprints = fixture_fingerprints(&MIXED_HASHES);
    let families = [family(MIXED_MEMBERS, MIXED_BRIDGE_EDGES)];
    let judge = HashAwareJudge {
        fingerprints: &fingerprints,
        calls: AtomicUsize::new(0),
    };
    let mut pending = existing_candidate(EXISTING_HASH_PAIR);
    pending.score.structural = NO_SCORE;
    pending.score.token_jaccard = crate::pair::SHARED_SUBTREE_MIN_JACCARD;
    let groups: Vec<_> = recovery_groups(&families, &fingerprints, &judge, &[pending]).collect();
    assert_eq!(groups, vec![FIRST_GROUP.to_vec()]);
    assert_eq!(judge.calls.load(Ordering::Relaxed), ONE_NEW_PAIR);
}

// [FUSED-CANDIDATE-BUCKET-RECOVERY] Skipping known edges must keep
// a third same-hash clone even when clone relations are nontransitive.
#[test]
fn recovery_keeps_a_missing_nontransitive_copy() {
    let fingerprints = fixture_fingerprints(&NONTRANSITIVE_HASHES);
    let families = [family(NONTRANSITIVE_MEMBERS, NONTRANSITIVE_EDGES)];
    let existing: Vec<_> = NONTRANSITIVE_EXISTING
        .iter()
        .copied()
        .map(existing_candidate)
        .collect();
    let recovered: Vec<_> =
        recovery_groups(&families, &fingerprints, &RelationJudge, &existing).collect();
    let recovered_edges = recoverable_edges(&recovered, &fingerprints);
    assert!(recovered_edges.contains(&OLD_RECOVERABLE_EDGE));
    assert!(NONTRANSITIVE_EXISTING
        .iter()
        .all(|edge| !recovered_edges.contains(edge)));
}

#[test]
fn hash_partition_preserves_old_recoverable_edges_with_nontransitive_kinds() {
    let fingerprints = fixture_fingerprints(&NONTRANSITIVE_HASHES);
    let families = [family(NONTRANSITIVE_MEMBERS, NONTRANSITIVE_EDGES)];
    let judge = RelationJudge;
    let original: Vec<_> = judge
        .groups(NONTRANSITIVE_MEMBERS)
        .into_iter()
        .map(|(members, _kind)| members)
        .collect();
    let partitioned: Vec<_> = recovery_groups(&families, &fingerprints, &judge, &[]).collect();
    let old_edges = recoverable_edges(&original, &fingerprints);
    let new_edges = recoverable_edges(&partitioned, &fingerprints);
    assert!(old_edges.contains(&OLD_RECOVERABLE_EDGE));
    assert!(new_edges.is_superset(&old_edges),
        "hash partition lost a same-hash copy that the previous recovery found: {old_edges:?} vs {new_edges:?}");
}
