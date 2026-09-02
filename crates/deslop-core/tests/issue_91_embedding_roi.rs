//! Regression coverage for GH #91 embedding ROI signal loss.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, ClusterBuildInputs},
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureIndex, SIGNATURE_LEN},
    pair::{candidate_pairs, cluster_by_transitive_closure, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD},
    state::{FileId, FileRegistry},
};

#[test]
fn issue_91_embedding_only_pair_survives_when_lsh_misses_match() -> Result<()> {
    let (fingerprints, signatures) = low_jaccard_fixture();
    let lsh_pairs = Vec::new();
    let embedding_pairs = vec![EmbeddingPair {
        left: 0,
        right: 1,
        cosine: 0.99,
    }];

    let signature_index = SignatureIndex::from_slice(&signatures);
    let candidates = candidate_pairs(
        &fingerprints,
        &signature_index,
        &lsh_pairs,
        &embedding_pairs,
    );
    assert_eq!(candidates.len(), 1, "expected one fused candidate pair");
    let candidate = *candidates.first().context("one candidate pair expected")?;
    assert_eq!((candidate.left, candidate.right), (0, 1));
    assert!(
        lsh_pairs.is_empty(),
        "fixture must prove a unique embedding-only cluster"
    );
    assert!(
        candidate.score.structural.abs() < f64::EPSILON,
        "structural signal must be exactly zero in this fixture"
    );
    assert!(
        candidate.score.token_jaccard < LSH_ONLY_MIN_JACCARD,
        "fixture must exercise a match that LSH/token scoring missed"
    );
    assert!(
        candidate.score.embedding_cos > 0.98,
        "embedding cosine should be retained on overlapped LSH pairs"
    );
    assert!(
        candidate.score.bounded_fused() >= FUSED_THRESHOLD,
        "AI evidence should be enough to clear the fused threshold"
    );

    let clusters = cluster_by_transitive_closure(&candidates);
    assert_eq!(
        clusters.len(),
        1,
        "issue #91: high-confidence embedding-only evidence must produce a cluster"
    );
    let cluster = clusters.first().context("one cluster expected")?;
    assert_eq!(cluster.members, vec![0, 1]);

    // [FUSED-RANK-MASS] Pair evidence admitted the component above; the
    // materialised cluster owns only membership and duplicated mass.
    let rendered = build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: &fingerprints,
        fused_clusters: &clusters,
        trees: &[],
        file_languages: &HashMap::new(),
        file_paths: &HashMap::new(),
    });
    assert_eq!(
        rendered.len(),
        1,
        "the embedding-only cluster must survive materialisation"
    );
    let rendered_cluster = rendered.first().context("one rendered cluster expected")?;
    assert_eq!(rendered_cluster.members.len(), 2);
    assert_eq!(rendered_cluster.mass, 80);
    Ok(())
}

fn low_jaccard_fixture() -> (Vec<Fingerprint>, Vec<Signature>) {
    let mut registry = FileRegistry::new();
    let left = registry.register(PathBuf::from("left.py"));
    let right = registry.register(PathBuf::from("right.py"));
    (
        vec![fingerprint(left, 0, 80), fingerprint(right, 1, 80)],
        vec![[0; SIGNATURE_LEN], [1; SIGNATURE_LEN]],
    )
}

fn fingerprint(file_id: FileId, index: usize, node_count: usize) -> Fingerprint {
    Fingerprint {
        hash: [u8::try_from(index).unwrap_or_default(); 32],
        file_id,
        byte_range: ByteRange {
            start: index.saturating_mul(100),
            end: index.saturating_mul(100).saturating_add(50),
        },
        node_count,
    }
}
