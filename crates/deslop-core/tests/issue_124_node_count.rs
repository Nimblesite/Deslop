//! Final-contract replacement for GH #124's retired confidence-weighted
//! ranking: duplicated mass outranks confidence ([RANK-MASS-SUM]).
//!
//! The fixture no longer declares a signal triple: under
//! [FUSED-CLUSTER-SIGNALS] a cluster's signals are measured between the
//! occurrences the report renders, so the fixture must supply the
//! evidence that produces them — identical Merkle hashes for the exact
//! pair, partially agreeing `MinHash` signatures for the Type-4 pair, and
//! embedding vectors only where the pass actually produced them.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs},
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureIndex, SIGNATURE_LEN},
    pair::FusedCluster,
    pair::FusedEdge,
    state::{FileId, FileRegistry},
};

/// Positions the Type-4 signatures agree on: 46/128 == 0.359375.
const TYPE4_AGREEMENTS: usize = 46;
/// Measured Jaccard the Type-4 signature pair must yield.
const TYPE4_JACCARD: f64 = 0.359_375;
/// Cosine the Type-4 embedding vectors encode.
const TYPE4_COSINE: f64 = 0.94;
/// Float slack for measured signal comparisons (`f32` vector arithmetic).
const SIGNAL_TOLERANCE: f64 = 1e-5;

#[test]
fn rank_mass_never_discounts_a_large_semantic_clone_by_confidence() -> Result<()> {
    let fixture = type4_weight_fixture();

    let signature_index = SignatureIndex::from_slice(&fixture.signatures);
    let clusters = build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: &fixture.fingerprints,
        signatures: &signature_index,
        embedding_vectors: &fixture.vectors,
        fused_clusters: &fixture.fused_clusters,
        trees: &[],
        sources: &HashMap::new(),
        file_languages: &HashMap::new(),
        file_paths: &HashMap::new(),
    });
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
    assert_eq!(semantic.weight, 814.0, "semantic duplicated mass is exact");
    assert_eq!(exact.weight, 182.0, "exact duplicated mass is exact");
    assert!(semantic.weight > exact.weight, "confidence never discounts mass");
    assert_eq!(
        clusters.first().map(|cluster| cluster.id.as_str()),
        Some(semantic.id.as_str()),
        "the larger duplicated mass ranks first regardless of confidence"
    );

    assert_measured_signals(semantic, exact);

    Ok(())
}

/// [FUSED-CLUSTER-SIGNALS]: every rendered signal is measured between
/// the rendered occurrences, so each one must equal the evidence the
/// fixture supplied — not a discovery-edge average.
fn assert_measured_signals(semantic: &Cluster, exact: &Cluster) {
    assert!(
        semantic.signals.structural.abs() < f64::EPSILON,
        "Type-4 members carry different Merkle hashes, so measured structural is exactly 0.0; got {}",
        semantic.signals.structural
    );
    assert!(
        (semantic.signals.token_jaccard - TYPE4_JACCARD).abs() < SIGNAL_TOLERANCE,
        "Type-4 token signal must be the MinHash estimate of the two rendered signatures ({TYPE4_JACCARD}); got {}",
        semantic.signals.token_jaccard
    );
    assert!(
        (semantic.signals.embedding_cos - TYPE4_COSINE).abs() < SIGNAL_TOLERANCE,
        "Type-4 cosine must be measured between the two rendered vectors ({TYPE4_COSINE}); got {}",
        semantic.signals.embedding_cos
    );
    assert!(
        (exact.signals.structural - 1.0).abs() < f64::EPSILON,
        "byte-identical members must measure structural exactly 1.0; got {}",
        exact.signals.structural
    );
    assert!(
        (exact.signals.token_jaccard - 1.0).abs() < f64::EPSILON,
        "identical signatures must measure Jaccard exactly 1.0; got {}",
        exact.signals.token_jaccard
    );
    assert!(
        exact.signals.embedding_cos.abs() < f64::EPSILON,
        "the exact pair has no vectors, so its cosine is unmeasured and renders 0.0 — never a stand-in value; got {}",
        exact.signals.embedding_cos
    );
}

/// Everything `build_ranked_fused_clusters` needs to measure the fixture.
struct Type4Fixture {
    /// Corpus fingerprints, indexed by fused-cluster member index.
    fingerprints: Vec<Fingerprint>,
    /// `MinHash` signatures parallel to `fingerprints`.
    signatures: Vec<Signature>,
    /// Embedding vectors by fingerprint index; absent where the pass
    /// produced none.
    vectors: HashMap<usize, Vec<f32>>,
    /// Transitive-closure output under measurement.
    fused_clusters: Vec<FusedCluster>,
}

fn type4_weight_fixture() -> Type4Fixture {
    let mut registry = FileRegistry::new();
    let semantic_left = registry.register(PathBuf::from("fly.py"));
    let semantic_right = registry.register(PathBuf::from("docker_host.py"));
    let exact_left = registry.register(PathBuf::from("auth_a.py"));
    let exact_right = registry.register(PathBuf::from("auth_b.py"));

    Type4Fixture {
        fingerprints: vec![
            fingerprint(semantic_left, 1, 814, 0, 900),
            fingerprint(semantic_right, 2, 814, 1_000, 1_900),
            // An exact duplicate is exact: both occurrences hash alike.
            fingerprint(exact_left, 3, 182, 2_000, 2_200),
            fingerprint(exact_right, 3, 182, 2_300, 2_500),
        ],
        signatures: vec![
            signature(11, SIGNATURE_LEN, 0),
            signature(11, TYPE4_AGREEMENTS, 97),
            signature(23, SIGNATURE_LEN, 0),
            signature(23, SIGNATURE_LEN, 0),
        ],
        // Only the Type-4 pair was embedded; the exact pair was found
        // structurally, with no vectors to measure.
        vectors: HashMap::from([(0, vec![1.0, 0.0]), (1, vec![0.94, 0.341_174_44])]),
        fused_clusters: vec![
            FusedCluster {
                members: vec![0, 1],
                edges: vec![FusedEdge {
                    left: 0,
                    right: 1,
                    strength: TYPE4_COSINE,
                }],
            },
            FusedCluster {
                members: vec![2, 3],
                edges: vec![FusedEdge {
                    left: 2,
                    right: 3,
                    strength: 1.0,
                }],
            },
        ],
    }
}

/// Builds a signature of `base` values whose tail past `agreements` is
/// overwritten with `filler`, so two signatures sharing `base` agree on
/// exactly `agreements` of [`SIGNATURE_LEN`] positions.
fn signature(base: u64, agreements: usize, filler: u64) -> Signature {
    let mut signature = [base; SIGNATURE_LEN];
    for slot in signature.iter_mut().skip(agreements) {
        *slot = filler;
    }
    signature
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
