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
        if admits_cosine(cosine) {
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
///
/// Both the normalisation and the dot product run in `f32`. At four
/// digits of vector width that accumulates about `2e-6` of drift, so two
/// byte-identical snippets sharing one vector report `0.999998` rather
/// than `1.0`. Harmless at the current four lanes, load-bearing at any
/// realistic width — GH #369.
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
        if !admits_cosine(cosine) {
            continue;
        }
        out.push(order_pair(query_index, neighbour, cosine));
    }
}

/// Returns `true` when a measured cosine is admissible evidence.
///
/// Finiteness is checked first and separately from the floor, because a
/// non-finite cosine passes any `<` test by definition: written as
/// `cosine < MIN_COSINE`, the ANN filter kept every `NaN` neighbour it
/// was handed. `NaN` reaches here whenever a component overflows `f32`,
/// so a malformed provider response would manufacture pairs rather than
/// be discarded. Both the exact and ANN paths route through this one
/// predicate so neither can drift open again.
fn admits_cosine(cosine: f64) -> bool {
    cosine.is_finite() && cosine >= MIN_COSINE
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Vector widths spanning the mock fixtures (4) and the widths real
    /// embedding models return, where the accumulated error is largest.
    const WIDTHS: [usize; 4] = [4, 384, 768, 4096];

    /// Deterministic non-dyadic components, so L2 normalisation cannot be
    /// exact in binary floating point and the rounding under test is
    /// actually exercised.
    fn ramp(width: usize) -> Vec<f32> {
        (0..width)
            .map(|index| {
                let step = u16::try_from(index % 997).unwrap_or_default();
                0.1_f32 + f32::from(step) * 0.017_f32
            })
            .collect()
    }

    /// [FUSION-EMBED-PROVIDER] A vector is perfectly similar to itself. Two
    /// byte-identical snippets share one vector (`group_snippets_by_content`
    /// collapses them), so this is exactly the figure the report renders for
    /// an identical clone pair — it must be `1.0`, not `0.999998`. GH #372.
    #[test]
    fn identical_vectors_have_cosine_similarity_of_exactly_one() {
        for width in WIDTHS {
            let vector = ramp(width);
            let cosine = cosine_similarity(&vector, &vector);
            assert!(
                (cosine - 1.0).abs() < f64::EPSILON,
                "a vector of width {width} must be perfectly similar to itself, got {cosine:.17}",
            );
        }
    }

    /// [FUSION-EMBED-PROVIDER] Scaling a vector does not change its
    /// direction, so cosine stays exactly `1.0` regardless of magnitude.
    #[test]
    fn scaled_copies_of_a_vector_have_cosine_similarity_of_exactly_one() {
        for width in WIDTHS {
            let vector = ramp(width);
            let scaled: Vec<f32> = vector.iter().map(|value| value * 3.5).collect();
            let cosine = cosine_similarity(&vector, &scaled);
            assert!(
                (cosine - 1.0).abs() < f64::EPSILON,
                "a scaled copy at width {width} must stay perfectly similar, got {cosine:.17}",
            );
        }
    }

    /// [FUSION-EMBED-PROVIDER] The accurate path still reports the analytic
    /// answers for orthogonal, opposed, and degenerate inputs, so tightening
    /// precision cannot be mistaken for widening admission.
    #[test]
    fn cosine_similarity_reports_analytic_values_for_known_vectors() {
        assert!(
            cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < f64::EPSILON,
            "orthogonal vectors must score exactly 0.0",
        );
        assert!(
            cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]).abs() < f64::EPSILON,
            "opposed vectors must clamp to exactly 0.0",
        );
        assert!(
            cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]).abs() < f64::EPSILON,
            "a zero-norm vector must score exactly 0.0",
        );
        let half = cosine_similarity(&[1.0, 0.0], &[1.0, 1.0]);
        assert!(
            (half - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9,
            "45 degrees apart must score 1/sqrt(2), got {half:.17}",
        );
    }
}
