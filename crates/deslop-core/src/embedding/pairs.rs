//! Embedding ANN pair generator.
//!
//! Implements the embedding arm of [FUSION-EMBED-PROVIDER]: build an
//! HNSW index over per-fingerprint embeddings, retrieve top-k
//! cosine-nearest neighbours per query, and emit pairs whose
//! `embedding_cos` clears a threshold. Callers (see `pipeline`) union
//! the resulting pairs with the structural + LSH pairs before fusion.

use instant_distance::{Builder, Point, Search};

use crate::fingerprint::Fingerprint;

/// Number of nearest neighbours retrieved per query. Small because
/// the pair stream is later unioned with the structural + LSH pairs;
/// recall comes from the union, not from the ANN fan-out alone.
const TOP_K: usize = 5;
/// Maximum corpus size where exact pair scoring is cheaper and more
/// reliable than ANN recall. Small fixture and edited-file runs can have
/// many near-tied subtree embeddings; exact scoring prevents top-k
/// neighbour truncation from dropping the only declaration-level Type-4
/// pair.
const EXACT_PAIR_LIMIT: usize = 256;
/// Minimum cosine similarity required for a pair to count as an
/// embedding candidate. The fused threshold in [`crate::pair`] still
/// applies; this gate keeps noisy low-cosine neighbours out of the
/// candidate set so clustering does not drown in weak signal.
const MIN_COSINE: f64 = 0.80;
/// Deterministic HNSW construction seed. Any fixed value works; the
/// hex bytes spell `codede` twice so operators can recognise it in
/// logs.
const HNSW_SEED: u64 = 0xC0DE_DEC0_DEDE_C0DE_u64;

/// A single fingerprint pair produced by the ANN pass with its
/// cosine similarity. `left < right` so pairs are order-insensitive.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddingPair {
    /// Lower fingerprint index.
    pub left: usize,
    /// Higher fingerprint index.
    pub right: usize,
    /// Cosine similarity in `[0, 1]` (negative cosines are clamped).
    pub cosine: f64,
}

/// Builds an HNSW over `embeddings` and returns the top-k nearest
/// neighbour pairs whose cosine clears [`MIN_COSINE`]. Each index in
/// `embeddings` corresponds one-to-one with the `fingerprints` slice
/// provided to the pipeline; returned pair indices are therefore
/// usable directly as fingerprint indices.
#[must_use]
pub fn embedding_pairs(
    fingerprints: &[Fingerprint],
    embeddings: &[Vec<f32>],
) -> Vec<EmbeddingPair> {
    if embeddings.len() != fingerprints.len() || embeddings.len() < 2 {
        return Vec::new();
    }
    let ann_pairs = ann_embedding_pairs(embeddings);
    if embeddings.len() <= EXACT_PAIR_LIMIT {
        let mut pairs = ann_pairs;
        pairs.extend(exact_embedding_pairs(embeddings));
        return dedupe(pairs);
    }
    ann_pairs
}

/// Retrieves top-k ANN neighbours for every embedding.
fn ann_embedding_pairs(embeddings: &[Vec<f32>]) -> Vec<EmbeddingPair> {
    let points: Vec<CosinePoint> = embeddings
        .iter()
        .map(|vector| CosinePoint::new(vector))
        .collect();
    let indices: Vec<usize> = (0..embeddings.len()).collect();
    let map = Builder::default().seed(HNSW_SEED).build(points, indices);
    let mut search = Search::default();
    let mut pairs: Vec<EmbeddingPair> = Vec::new();
    for (query_index, query) in embeddings.iter().enumerate() {
        let probe = CosinePoint::new(query);
        collect_neighbours(&map, &probe, query_index, &mut search, &mut pairs);
    }
    dedupe(pairs)
}

/// Scores every pair exactly for small corpora where ANN top-k recall is
/// more fragile than the quadratic work is expensive.
fn exact_embedding_pairs(embeddings: &[Vec<f32>]) -> Vec<EmbeddingPair> {
    let points: Vec<CosinePoint> = embeddings
        .iter()
        .map(|vector| CosinePoint::new(vector))
        .collect();
    let mut pairs = Vec::new();
    for left in 0..points.len() {
        collect_exact_pairs_from(left, &points, &mut pairs);
    }
    pairs
}

/// Appends exact embedding candidates for one left endpoint.
fn collect_exact_pairs_from(left: usize, points: &[CosinePoint], pairs: &mut Vec<EmbeddingPair>) {
    let Some(left_point) = points.get(left) else {
        return;
    };
    for right in left.saturating_add(1)..points.len() {
        let Some(right_point) = points.get(right) else {
            continue;
        };
        let cosine = cosine_between(left_point, right_point);
        if cosine >= MIN_COSINE {
            pairs.push(EmbeddingPair {
                left,
                right,
                cosine,
            });
        }
    }
}

/// Returns cosine similarity for two already-normalised points.
fn cosine_between(left: &CosinePoint, right: &CosinePoint) -> f64 {
    cosine_from_distance(f64::from(left.distance(right)))
}

/// Returns the cosine similarity of two raw vectors in `[0, 1]`.
///
/// This is the crate's single definition of cosine similarity: the same
/// L2 normalisation, dot product, and negative-cosine clamp the ANN pass
/// applies to every [`EmbeddingPair`]. Cluster-level signal measurement
/// calls this so a rendered `embedding_cos` is always computed by the
/// identical arithmetic that admitted the pair evidence — a second,
/// subtly different cosine would let the report disagree with the
/// pipeline about the same two vectors.
#[must_use]
pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    cosine_between(&CosinePoint::new(left), &CosinePoint::new(right))
}

/// Runs a single HNSW query and appends any surviving pairs to `out`.
fn collect_neighbours(
    map: &instant_distance::HnswMap<CosinePoint, usize>,
    probe: &CosinePoint,
    query_index: usize,
    search: &mut Search,
    out: &mut Vec<EmbeddingPair>,
) {
    for hit in map.search(probe, search).take(TOP_K) {
        let neighbour = *hit.value;
        if neighbour == query_index {
            continue;
        }
        let cosine = cosine_from_distance(f64::from(hit.distance));
        if cosine < MIN_COSINE {
            continue;
        }
        out.push(order_pair(query_index, neighbour, cosine));
    }
}

/// Converts instant-distance cosine **distance** back to a cosine
/// **similarity** in `[0, 1]`. Negative cosines (possible when
/// embeddings are not strictly non-negative) are clamped to zero so
/// the value composes with `token_jaccard` and `structural` in the
/// fused sum.
fn cosine_from_distance(distance: f64) -> f64 {
    (1.0 - distance).clamp(0.0, 1.0)
}

/// Returns an [`EmbeddingPair`] with `left < right` so pair equality
/// is order-insensitive.
fn order_pair(a: usize, b: usize, cosine: f64) -> EmbeddingPair {
    if a <= b {
        EmbeddingPair {
            left: a,
            right: b,
            cosine,
        }
    } else {
        EmbeddingPair {
            left: b,
            right: a,
            cosine,
        }
    }
}

/// Keeps the highest cosine for each (left, right) pair. HNSW can
/// surface the same pair twice (once from each endpoint's query); we
/// keep the better score so fusion has consistent input.
fn dedupe(mut pairs: Vec<EmbeddingPair>) -> Vec<EmbeddingPair> {
    pairs.sort_by(|lhs, rhs| {
        lhs.left
            .cmp(&rhs.left)
            .then_with(|| lhs.right.cmp(&rhs.right))
            .then_with(|| {
                rhs.cosine
                    .partial_cmp(&lhs.cosine)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    pairs.dedup_by(|later, earlier| later.left == earlier.left && later.right == earlier.right);
    pairs
}

/// HNSW point with cosine distance. Pre-normalises the vector at
/// construction time so the hot path in `distance` is a single dot
/// product.
#[derive(Clone)]
struct CosinePoint {
    /// L2-normalised copy of the input vector. Zero-norm inputs
    /// become all-zero so their cosine with anything is zero.
    vector: Vec<f32>,
}

impl CosinePoint {
    /// Allocates an L2-normalised copy of `values`.
    fn new(values: &[f32]) -> Self {
        let mut norm_sq: f32 = 0.0;
        for value in values {
            norm_sq += value.mul_add(*value, 0.0);
        }
        let norm = norm_sq.sqrt();
        if norm <= f32::EPSILON {
            return Self {
                vector: vec![0.0_f32; values.len()],
            };
        }
        let vector: Vec<f32> = values.iter().map(|value| value / norm).collect();
        Self { vector }
    }
}

impl Point for CosinePoint {
    fn distance(&self, other: &Self) -> f32 {
        let mut dot: f32 = 0.0;
        for (left, right) in self.vector.iter().zip(other.vector.iter()) {
            dot = left.mul_add(*right, dot);
        }
        // Pre-normalised vectors → cosine = dot → distance = 1 - dot.
        1.0_f32 - dot
    }
}
