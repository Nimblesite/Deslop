//! GH #124: cluster ranking is duplicated mass and nothing else.

use std::collections::HashMap;

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, ClusterBuildInputs},
    fingerprint::Fingerprint,
    pair::FusedCluster,
    state::{FileId, FileRegistry},
};

/// Node count of the larger cluster.
const LARGE_NODES: usize = 814;
/// Node count of the smaller cluster.
const SMALL_NODES: usize = 182;

#[test]
fn rank_mass_orders_by_extent_times_additional_occurrences() {
    let mut registry = FileRegistry::new();
    let file_ids: Vec<FileId> = (0..4)
        .map(|index| registry.register(format!("file-{index}.rs").into()))
        .collect();
    let fingerprints = vec![
        member(file_ids[0], LARGE_NODES, 1),
        member(file_ids[1], LARGE_NODES, 2),
        member(file_ids[2], SMALL_NODES, 3),
        member(file_ids[3], SMALL_NODES, 4),
    ];
    let fused = [
        FusedCluster {
            members: vec![0, 1],
            edges: Vec::new(),
        },
        FusedCluster {
            members: vec![2, 3],
            edges: Vec::new(),
        },
    ];
    let clusters = build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: &fingerprints,
        fused_clusters: &fused,
        trees: &[],
        file_languages: &HashMap::new(),
        file_paths: &HashMap::new(),
    });
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].mass, u64::try_from(LARGE_NODES).unwrap_or(u64::MAX));
    assert_eq!(clusters[1].mass, u64::try_from(SMALL_NODES).unwrap_or(u64::MAX));
    assert!(clusters[0].mass > clusters[1].mass);
}

/// Builds one exact member with a distinct file and digest.
fn member(file_id: FileId, nodes: usize, digest: u8) -> Fingerprint {
    Fingerprint {
        hash: [digest; 32],
        file_id,
        byte_range: ByteRange { start: 0, end: nodes },
        node_count: nodes,
    }
}
