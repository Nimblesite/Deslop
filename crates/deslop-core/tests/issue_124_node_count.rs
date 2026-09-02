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
fn rank_mass_orders_by_extent_times_additional_occurrences() -> Result<(), &'static str> {
    let mut registry = FileRegistry::new();
    let [large_left, large_right, small_left, small_right]: [FileId; 4] =
        std::array::from_fn(|index| registry.register(format!("file-{index}.rs").into()));
    let fingerprints = vec![
        member(large_left, LARGE_NODES, 1),
        member(large_right, LARGE_NODES, 2),
        member(small_left, SMALL_NODES, 3),
        member(small_right, SMALL_NODES, 4),
    ];
    let fused = [
        FusedCluster {
            members: vec![0, 1],
            edges: Vec::new(),
            shape_family: None,
        },
        FusedCluster {
            members: vec![2, 3],
            edges: Vec::new(),
            shape_family: None,
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
    let larger = clusters.first().ok_or("larger cluster must rank first")?;
    let smaller = clusters.get(1).ok_or("smaller cluster must rank second")?;
    assert_eq!(larger.mass, u64::try_from(LARGE_NODES).unwrap_or(u64::MAX));
    assert_eq!(smaller.mass, u64::try_from(SMALL_NODES).unwrap_or(u64::MAX));
    assert!(larger.mass > smaller.mass);
    Ok(())
}

/// Builds one exact member with a distinct file and digest.
fn member(file_id: FileId, nodes: usize, digest: u8) -> Fingerprint {
    Fingerprint {
        hash: [digest; 32],
        file_id,
        byte_range: ByteRange {
            start: 0,
            end: nodes,
        },
        node_count: nodes,
    }
}
