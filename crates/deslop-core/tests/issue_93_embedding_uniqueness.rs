//! Regression coverage for GH #93: the embedding pass must recall
//! clusters the LSH pass missed.
//!
//! This suite used to assert the inverse of [REPAIR-COSINE-MERGE] (#351)
//! — that a pair LSH already found is denied its ANN cosine, to preserve
//! "unique-recall accounting". No consumer of that accounting ever
//! existed, and the denial made discovery route decide evidence: the same
//! two files scored differently depending on which pass reached them
//! first, and a byte-identical pair found structurally rendered
//! `embedding_cos = 0.0` — indistinguishable from "measured and found
//! unrelated". The assertion is now inverted and tightened: every
//! measured cosine is credited to its pair exactly, whatever surfaced it.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, ClusterBuildInputs},
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureIndex, SIGNATURE_LEN},
    pair::{
        candidate_pairs, cluster_by_transitive_closure, FusedCluster, LSH_ONLY_MIN_JACCARD,
        LSH_ONLY_MIN_NODE_COUNT,
    },
    state::{FileId, FileRegistry},
};

#[test]
fn issue_93_embedding_pass_recalls_lsh_missed_clusters_and_credits_every_cosine() -> Result<()> {
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

    let signature_index = SignatureIndex::from_slice(&signatures);
    let candidates = candidate_pairs(
        &fingerprints,
        &signature_index,
        &lsh_pairs,
        &embedding_pairs,
    );
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
    // [REPAIR-COSINE-MERGE] #351: a measured cosine is evidence about the
    // pair; the pass that reached it first is telemetry. This pair was
    // surfaced by LSH *and* measured by the ANN pass, so the LSH hit must
    // not erase the cosine.
    assert!(
        (lsh_visible.score.embedding_cos - 0.99).abs() < f64::EPSILON,
        "issue #351: the ANN cosine must be merged into a pair LSH already found, not discarded; got {}",
        lsh_visible.score.embedding_cos
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
    assert!(
        (embedding_only.score.embedding_cos - 0.98).abs() < f64::EPSILON,
        "the ANN-only pair must carry exactly its measured cosine; got {}",
        embedding_only.score.embedding_cos
    );
    // Discovery-route invariance: both pairs carry precisely the cosine
    // their `EmbeddingPair` declared. One was also found by LSH and one
    // was not, and that difference changes nothing about the evidence.
    assert!(
        (lsh_visible.score.embedding_cos - embedding_only.score.embedding_cos - 0.01).abs() < 1e-9,
        "issue #351: both pairs must keep their own measured cosine (0.99 vs 0.98); got {} and {}",
        lsh_visible.score.embedding_cos,
        embedding_only.score.embedding_cos
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
    assert_eq!(
        unique_cluster.members,
        vec![2, 3],
        "the LSH-missed pair must remain its own component"
    );

    assert_materialised_clusters_are_mass_only(&fingerprints, &clusters)
}

/// [FUSED-RANK-MASS] Pair evidence stays on the candidate pairs; a
/// materialised component contains membership and duplicated mass only.
fn assert_materialised_clusters_are_mass_only(
    fingerprints: &[Fingerprint],
    clusters: &[FusedCluster],
) -> Result<()> {
    let rendered = build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints,
        fused_clusters: clusters,
        trees: &[],
        file_languages: &HashMap::new(),
        file_paths: &HashMap::new(),
    });
    assert_eq!(
        rendered.len(),
        2,
        "both clusters must survive materialisation"
    );
    let rendered_unique = rendered.iter().find(|cluster| {
        cluster.members.iter().any(|member| member.hash == [2; 32])
    })
        .context("expected the rendered embedding-only cluster")?;
    assert_eq!(rendered_unique.members.len(), 2);
    assert_eq!(rendered_unique.mass, LSH_ONLY_MIN_NODE_COUNT);

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
