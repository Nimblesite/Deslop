//! Regression coverage for GH #93 embedding unique-recall accounting.

use std::path::PathBuf;

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SIGNATURE_LEN},
    pair::{
        candidate_pairs, cluster_by_transitive_closure, LSH_ONLY_MIN_JACCARD,
        LSH_ONLY_MIN_NODE_COUNT,
    },
    state::{FileId, FileRegistry},
};

#[test]
fn issue_93_embedding_signal_only_marks_pairs_that_lsh_missed() -> Result<()> {
    let (fingerprints, signatures) = embedding_roi_fixture();
    let lsh_pairs = vec![(0, 1)];
    let embedding_pairs = vec![
        EmbeddingPair {
            left: 0,
            right: 1,
            cosine: 0.99,
        },
        EmbeddingPair {
            left: 2,
            right: 3,
            cosine: 0.98,
        },
    ];

    let candidates = candidate_pairs(&fingerprints, &signatures, &lsh_pairs, &embedding_pairs);
    assert_eq!(
        candidates.len(),
        2,
        "fixture should produce two candidate pairs"
    );

    let lsh_visible = candidates
        .iter()
        .find(|pair| (pair.left, pair.right) == (0, 1))
        .context("expected LSH-visible pair")?;
    assert!(
        lsh_visible.score.token_jaccard >= LSH_ONLY_MIN_JACCARD,
        "fixture must prove this pair was already visible to LSH"
    );
    assert!(
        lsh_visible.score.embedding_cos.abs() < f64::EPSILON,
        "issue #93: embedding signal must not be credited to pairs LSH already found"
    );

    let embedding_only = candidates
        .iter()
        .find(|pair| (pair.left, pair.right) == (2, 3))
        .context("expected embedding-only pair")?;
    assert!(
        embedding_only.score.token_jaccard < LSH_ONLY_MIN_JACCARD,
        "fixture must prove this pair was missed by LSH"
    );
    assert!(
        embedding_only.score.embedding_cos > 0.95,
        "embedding-only pair should retain high cosine evidence"
    );
    assert!(
        embedding_only.score.bounded_fused() >= 0.85,
        "embedding-only evidence should still survive fusion"
    );

    let clusters = cluster_by_transitive_closure(&candidates);
    assert_eq!(
        clusters.len(),
        2,
        "both candidate pairs should survive separately"
    );
    let unique_cluster = clusters
        .iter()
        .find(|cluster| cluster.members == vec![2, 3])
        .context("expected cluster for embedding-only pair")?;
    assert!(
        unique_cluster.mean_score.embedding_cos > 0.95,
        "issue #93: embedding pass must keep at least one LSH-missed cluster"
    );

    Ok(())
}

fn embedding_roi_fixture() -> (Vec<Fingerprint>, Vec<Signature>) {
    let mut registry = FileRegistry::new();
    let lsh_left = registry.register(PathBuf::from("lsh_left.py"));
    let lsh_right = registry.register(PathBuf::from("lsh_right.py"));
    let semantic_left = registry.register(PathBuf::from("semantic_left.py"));
    let semantic_right = registry.register(PathBuf::from("semantic_right.py"));
    (
        vec![
            fingerprint(lsh_left, 0),
            fingerprint(lsh_right, 1),
            fingerprint(semantic_left, 2),
            fingerprint(semantic_right, 3),
        ],
        vec![
            [7; SIGNATURE_LEN],
            [7; SIGNATURE_LEN],
            [11; SIGNATURE_LEN],
            [19; SIGNATURE_LEN],
        ],
    )
}

fn fingerprint(file_id: FileId, index: usize) -> Fingerprint {
    Fingerprint {
        hash: [u8::try_from(index).unwrap_or_default(); 32],
        file_id,
        byte_range: ByteRange {
            start: index.saturating_mul(100),
            end: index.saturating_mul(100).saturating_add(50),
        },
        node_count: LSH_ONLY_MIN_NODE_COUNT,
    }
}
