use std::collections::HashMap;

use super::*;
use crate::{ast::ByteRange, pair::FusedEdge, state::FileRegistry};

const HASH_FIRST: [u8; 32] = [1; 32];
const HASH_SECOND: [u8; 32] = [2; 32];
const HASH_SINGLETON: [u8; 32] = [3; 32];
const FIXTURE_PATH: &str = "mixed.rs";
const FIXTURE_START: usize = 0;
const FIXTURE_END: usize = 1;
const FIXTURE_NODES: usize = 40;
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
