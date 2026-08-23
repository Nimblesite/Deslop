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
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs},
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SIGNATURE_LEN},
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

    assert_rendered_signals_are_measured(&fingerprints, &signatures, &clusters)
}

/// [FUSION-CLUSTER-SIGNALS] Rendered signals are measured between the
/// rendered occurrences. The LSH-visible pair's *discovery edge*
/// deliberately withholds embedding credit (asserted by the caller), but
/// the report still shows the true cosine of the two vectors — discovery
/// bookkeeping never becomes a rendered figure.
fn assert_rendered_signals_are_measured(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    clusters: &[FusedCluster],
) -> Result<()> {
    let vectors = HashMap::from([
        (0, vec![1.0, 0.0]),
        (1, vec![0.99, 0.141_067_36]),
        (2, vec![1.0, 0.0]),
        (3, vec![0.98, 0.198_997_49]),
    ]);
    let rendered = build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints,
        signatures,
        embedding_vectors: &vectors,
        fused_clusters: clusters,
        trees: &[],
        sources: &HashMap::new(),
        file_languages: &HashMap::new(),
        file_paths: &HashMap::new(),
    });
    assert_eq!(
        rendered.len(),
        2,
        "both clusters must survive materialisation"
    );
    let rendered_unique = find_by_leading_hash(&rendered, 2)
        .context("expected the rendered embedding-only cluster")?;
    assert!(
        rendered_unique.signals.embedding_cos > 0.95,
        "issue #93: embedding pass must keep at least one LSH-missed cluster; got {}",
        rendered_unique.signals.embedding_cos
    );
    assert!(
        (rendered_unique.signals.embedding_cos - 0.98).abs() < 1e-5,
        "rendered cosine must equal the measured vector cosine (0.98); got {}",
        rendered_unique.signals.embedding_cos
    );
    assert!(
        rendered_unique.signals.token_jaccard.abs() < f64::EPSILON,
        "the LSH-missed cluster must render zero token evidence; got {}",
        rendered_unique.signals.token_jaccard
    );

    let rendered_lsh =
        find_by_leading_hash(&rendered, 0).context("expected the rendered LSH-visible cluster")?;
    assert!(
        (rendered_lsh.signals.token_jaccard - 1.0).abs() < f64::EPSILON,
        "identical signatures must render Jaccard exactly 1.0; got {}",
        rendered_lsh.signals.token_jaccard
    );
    assert!(
        (rendered_lsh.signals.embedding_cos - 0.99).abs() < 1e-5,
        "issue #93: withholding unique-recall credit on the discovery edge must not erase the measured cosine of the rendered pair; got {}",
        rendered_lsh.signals.embedding_cos
    );

    Ok(())
}

/// Finds a rendered cluster by the hash seed of its lowest member.
fn find_by_leading_hash(clusters: &[Cluster], seed: u8) -> Option<&Cluster> {
    clusters.iter().find(|cluster| {
        cluster
            .members
            .iter()
            .any(|member| member.hash == [seed; 32])
    })
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
