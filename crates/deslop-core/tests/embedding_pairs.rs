//! Public embedding-pair API coverage.

use std::path::PathBuf;

use anyhow::{Context, Result};
use deslop_core::{
    ast::ByteRange,
    embedding::{cosine_similarity, embedding_pairs, EmbeddingPair},
    fingerprint::Fingerprint,
    state::FileRegistry,
};

/// Documented ANN admission floor (`MIN_COSINE` in
/// `deslop-core/src/embedding/pairs.rs`): at or above this, a measured
/// cosine is admissible clone evidence.
const ADMISSION_FLOOR: f64 = 0.80;

/// Near-tied decoys crowding the query. More than `TOP_K` (5), so the
/// truncation is structural rather than a tie-break coincidence.
const DECOYS: usize = 10;

#[test]
fn embedding_pairs_dedupes_bidirectional_hits_and_filters_weak_neighbors() {
    let (fingerprints, embeddings) = fixture_vectors();
    let pairs = embedding_pairs(&fingerprints, &embeddings);
    assert_eq!(pairs.len(), 1, "expected one high-cosine pair: {pairs:?}");
    let pair = pairs.first().copied().unwrap_or(EmbeddingPair {
        left: usize::MAX,
        right: usize::MAX,
        cosine: 0.0,
    });
    assert_eq!((pair.left, pair.right), (0, 1));
    assert!(pair.cosine > 0.99, "cosine too low: {}", pair.cosine);
}

#[test]
fn embedding_pairs_rejects_mismatched_input_lengths() {
    let (fingerprints, mut embeddings) = fixture_vectors();
    let _dropped = embeddings.pop();
    assert!(embedding_pairs(&fingerprints, &embeddings).is_empty());
}

fn fixture_vectors() -> (Vec<Fingerprint>, Vec<Vec<f32>>) {
    let mut registry = FileRegistry::new();
    let fingerprints = (0..3)
        .map(|index| {
            let file_id = registry.register(PathBuf::from(format!("file-{index}.cs")));
            fingerprint(file_id, index)
        })
        .collect();
    let embeddings = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.999, 0.001, 0.0],
        vec![-1.0, 0.0, 0.0],
    ];
    (fingerprints, embeddings)
}

fn fingerprint(file_id: deslop_core::state::FileId, index: usize) -> Fingerprint {
    Fingerprint {
        hash: [u8::try_from(index).unwrap_or_default(); 32],
        file_id,
        byte_range: ByteRange {
            start: index.saturating_mul(10),
            end: index.saturating_mul(10).saturating_add(5),
        },
        node_count: 5,
    }
}

/// [FUSION-EMBED-PROVIDER] A pair whose measured cosine clears the
/// admission floor must reach fusion, even when more than `TOP_K` other
/// subtrees sit nearer the query.
///
/// The corpus is twelve embeddings — far inside the 256-vector
/// `EXACT_PAIR_LIMIT` that commit `31d5efd18` deleted along with
/// `exact_embedding_pairs`. Its own doc said why it existed: "small
/// fixture and edited-file runs can have many near-tied subtree
/// embeddings; exact scoring prevents top-k neighbour truncation from
/// dropping the only declaration-level Type-4 pair." Nothing in the tree
/// pinned that guarantee, so its removal is invisible until a corpus
/// like this one appears.
///
/// The dropped pair is the worst one to drop. A Type-4 clone — same
/// behaviour, different implementation — has no Merkle match and little
/// token overlap, so the structural and LSH passes the ANN stream is
/// unioned with cannot recover it. The embedding pass is its only route
/// into a report, and top-k truncation closes it.
#[test]
fn embedding_pairs_keeps_an_admissible_pair_that_top_k_neighbours_crowd_out() -> Result<()> {
    let (fingerprints, embeddings) = crowded_neighbourhood();
    let query = 0;
    let partner = embeddings.len().saturating_sub(1);
    let query_vector = embeddings.get(query).context("fixture query vector")?;
    let partner_vector = embeddings.get(partner).context("fixture partner vector")?;
    let measured = cosine_similarity(query_vector, partner_vector);
    assert!(
        measured >= ADMISSION_FLOOR,
        "fixture proves nothing unless the dropped pair is admissible: {measured}",
    );
    let pairs = embedding_pairs(&fingerprints, &embeddings);
    assert!(
        pairs.len() > DECOYS,
        "fixture must exercise the ANN pass: {pairs:?}",
    );
    assert!(
        pairs
            .iter()
            .any(|pair| (pair.left, pair.right) == (query, partner)),
        "the pair ({query}, {partner}) measures {measured} — admissible clone \
         evidence — but {DECOYS} near-tied neighbours crowd it out of every \
         top-k result, so it never reaches fusion. On a corpus this small the \
         deleted exact-pair path scored it directly. Returned pairs: {pairs:?}",
    );
    Ok(())
}

/// Twelve unit vectors: a query, `DECOYS` near-ties within five degrees
/// of it, and one genuine partner thirty degrees away.
///
/// Every decoy is nearer to *both* endpoints than the endpoints are to
/// each other, so neither endpoint's top-k query can reach the other.
fn crowded_neighbourhood() -> (Vec<Fingerprint>, Vec<Vec<f32>>) {
    let mut degrees = vec![0.0_f32];
    degrees.extend((1..=DECOYS).map(|step| 0.5_f32 * f32::from(u8::try_from(step).unwrap_or(0))));
    degrees.push(30.0_f32);
    let mut registry = FileRegistry::new();
    let fingerprints = (0..degrees.len())
        .map(|index| {
            let file_id = registry.register(PathBuf::from(format!("file-{index}.cs")));
            fingerprint(file_id, index)
        })
        .collect();
    let embeddings = degrees.into_iter().map(unit_vector).collect();
    (fingerprints, embeddings)
}

/// A two-dimensional unit vector at `degrees` from the first axis.
fn unit_vector(degrees: f32) -> Vec<f32> {
    let radians = degrees.to_radians();
    vec![radians.cos(), radians.sin()]
}
