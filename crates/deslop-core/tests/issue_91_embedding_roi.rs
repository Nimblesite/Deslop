//! Regression coverage for GH #91 embedding ROI signal loss.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::build_ranked_fused_clusters,
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SIGNATURE_LEN},
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

    let candidates = candidate_pairs(&fingerprints, &signatures, &lsh_pairs, &embedding_pairs);
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

    // [FUSION-CLUSTER-SIGNALS] The report must show the cosine measured
    // between the two rendered occurrences, not an average over the
    // discovery edges that assembled the component.
    let vectors = HashMap::from([(0, vec![1.0, 0.0]), (1, vec![0.99, 0.141_067_36])]);
    let rendered = build_ranked_fused_clusters(
        &fingerprints,
        &signatures,
        &vectors,
        &clusters,
        &[],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );
    assert_eq!(
        rendered.len(),
        1,
        "the embedding-only cluster must survive materialisation"
    );
    let rendered_cluster = rendered.first().context("one rendered cluster expected")?;
    assert!(
        rendered_cluster.signals.embedding_cos > 0.98,
        "issue #91: the rendered cluster must carry its embedding evidence; got {}",
        rendered_cluster.signals.embedding_cos
    );
    assert!(
        (rendered_cluster.signals.embedding_cos - 0.99).abs() < 1e-5,
        "rendered cosine must equal the measured vector cosine (0.99); got {}",
        rendered_cluster.signals.embedding_cos
    );
    assert!(
        rendered_cluster.signals.structural.abs() < f64::EPSILON,
        "distinct Merkle hashes must measure structural exactly 0.0; got {}",
        rendered_cluster.signals.structural
    );
    assert!(
        rendered_cluster.signals.token_jaccard.abs() < f64::EPSILON,
        "disjoint signatures must measure Jaccard exactly 0.0, proving this cluster is embedding-only in the report too; got {}",
        rendered_cluster.signals.token_jaccard
    );
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
