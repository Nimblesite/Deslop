//! Regression coverage for GH #124 Type-4 node-count ranking inflation.

use std::path::PathBuf;

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster},
    fingerprint::Fingerprint,
    pair::{FusedCluster, PairScore},
    state::{FileId, FileRegistry},
};

#[test]
fn issue_124_type4_node_count_does_not_dominate_refactor_ranking() -> Result<()> {
    let (fingerprints, fused_clusters) = type4_weight_fixture();

    let clusters = build_ranked_fused_clusters(&fingerprints, &fused_clusters);
    assert_eq!(
        clusters.len(),
        2,
        "fixture must produce two ranked clusters"
    );

    let semantic = clusters
        .iter()
        .find(|cluster| cluster.signals.embedding_cos > 0.90)
        .context("expected the Type-4 semantic cluster")?;
    let exact = clusters
        .iter()
        .find(|cluster| cluster.signals.structural > 0.99)
        .context("expected the exact structural cluster")?;

    assert!(
        semantic.signals.structural < 0.10,
        "fixture must model low-structural Type-4 evidence"
    );
    assert!(
        semantic.signals.embedding_cos > 0.90,
        "fixture must model high semantic evidence"
    );
    assert_eq!(
        smallest_node_count(semantic),
        814,
        "fixture must model the inflated Type-4 structural span"
    );
    assert_eq!(
        smallest_node_count(exact),
        182,
        "fixture must model the smaller actionable exact duplicate"
    );
    assert!(
        exact.weight > semantic.weight,
        "issue #124: low-structural Type-4 span should not outrank a smaller exact duplicate; semantic={} exact={}",
        semantic.weight,
        exact.weight
    );
    assert_eq!(
        clusters.first().map(|cluster| cluster.id.as_str()),
        Some(exact.id.as_str()),
        "issue #124: the actionable exact duplicate should be ranked first"
    );

    Ok(())
}

fn type4_weight_fixture() -> (Vec<Fingerprint>, Vec<FusedCluster>) {
    let mut registry = FileRegistry::new();
    let semantic_left = registry.register(PathBuf::from("fly.py"));
    let semantic_right = registry.register(PathBuf::from("docker_host.py"));
    let exact_left = registry.register(PathBuf::from("auth_a.py"));
    let exact_right = registry.register(PathBuf::from("auth_b.py"));

    let fingerprints = vec![
        fingerprint(semantic_left, 1, 814, 0, 900),
        fingerprint(semantic_right, 2, 814, 1_000, 1_900),
        fingerprint(exact_left, 3, 182, 2_000, 2_200),
        fingerprint(exact_right, 4, 182, 2_300, 2_500),
    ];
    let fused_clusters = vec![
        FusedCluster {
            members: vec![0, 1],
            mean_score: PairScore {
                structural: 0.02,
                token_jaccard: 0.36,
                embedding_cos: 0.94,
            },
        },
        FusedCluster {
            members: vec![2, 3],
            mean_score: PairScore {
                structural: 1.0,
                token_jaccard: 1.0,
                embedding_cos: 0.0,
            },
        },
    ];

    (fingerprints, fused_clusters)
}

fn fingerprint(
    file_id: FileId,
    hash_seed: u8,
    node_count: usize,
    start: usize,
    end: usize,
) -> Fingerprint {
    Fingerprint {
        hash: [hash_seed; 32],
        file_id,
        byte_range: ByteRange { start, end },
        node_count,
    }
}

fn smallest_node_count(cluster: &Cluster) -> usize {
    cluster
        .members
        .iter()
        .map(|member| member.node_count)
        .min()
        .unwrap_or_default()
}
