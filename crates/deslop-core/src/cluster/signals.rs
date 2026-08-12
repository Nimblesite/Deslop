//! Rendered-truth cluster signal measurement ([FUSION-CLUSTER-SIGNALS]).
//!
//! A cluster's signal triple is measured between the occurrences the
//! report actually shows — the per-signal mean over every unordered
//! pair of rendered occurrences. The surviving discovery edges of the
//! transitive-closure component are never averaged: their mix is an
//! artifact of discovery topology (structural star buckets, ANN top-k
//! fan-out, LSH band width), and under that mean a byte-identical file
//! pair once rendered `structural = 0.36` and routed `same_behavior`
//! (corpus, pinned by `issue_343_sum_clamp_saturation.rs`).

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    embedding::cosine_similarity,
    fingerprint::Fingerprint,
    lsh::{estimate_jaccard, Signature},
    pair::PairScore,
};

/// Measures the [FUSION-CLUSTER-SIGNALS] triple over the rendered
/// occurrence indices.
///
/// Per occurrence pair: `structural` is Merkle-hash equality,
/// `token_jaccard` is the `MinHash` estimate between the two signatures,
/// and `embedding_cos` is [`cosine_similarity`] of the two vectors —
/// the same arithmetic that admitted the ANN pair evidence. A pair
/// missing an input for a signal (no vector: embeddings off, oversized
/// input, provider failure) is excluded from that signal's numerator
/// and denominator both, so absence never masquerades as a measured
/// 0.0 inside the mean.
pub(super) fn measured_signals<S: BuildHasher>(
    occurrence_indices: &[usize],
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    embedding_vectors: &HashMap<usize, Vec<f32>, S>,
) -> PairScore {
    let mut totals = SignalTotals::default();
    for (position, &left) in occurrence_indices.iter().enumerate() {
        for &right in occurrence_indices.iter().skip(position.saturating_add(1)) {
            totals.add_pair(left, right, fingerprints, signatures, embedding_vectors);
        }
    }
    totals.means()
}

/// Per-signal running sums over the measurable occurrence pairs.
#[derive(Debug, Default)]
struct SignalTotals {
    /// Merkle-hash equality mean input.
    structural: MeanAccumulator,
    /// `MinHash` Jaccard mean input.
    token_jaccard: MeanAccumulator,
    /// Vector cosine mean input.
    embedding_cos: MeanAccumulator,
}

impl SignalTotals {
    /// Folds one occurrence pair into every signal it is measurable for.
    fn add_pair<S: BuildHasher>(
        &mut self,
        left: usize,
        right: usize,
        fingerprints: &[Fingerprint],
        signatures: &[Signature],
        embedding_vectors: &HashMap<usize, Vec<f32>, S>,
    ) {
        if let (Some(left_fp), Some(right_fp)) = (fingerprints.get(left), fingerprints.get(right)) {
            self.structural.add(if left_fp.hash == right_fp.hash {
                1.0
            } else {
                0.0
            });
        }
        if let (Some(left_sig), Some(right_sig)) = (signatures.get(left), signatures.get(right)) {
            self.token_jaccard
                .add(estimate_jaccard(left_sig, right_sig));
        }
        if let (Some(left_vec), Some(right_vec)) =
            (embedding_vectors.get(&left), embedding_vectors.get(&right))
        {
            self.embedding_cos
                .add(cosine_similarity(left_vec, right_vec));
        }
    }

    /// Returns the measured per-signal means.
    fn means(&self) -> PairScore {
        PairScore {
            structural: self.structural.mean(),
            token_jaccard: self.token_jaccard.mean(),
            embedding_cos: self.embedding_cos.mean(),
        }
    }
}

/// Sum + count pair yielding a mean of exactly the measured values.
#[derive(Debug, Default)]
struct MeanAccumulator {
    /// Sum of measured values.
    sum: f64,
    /// Count of measured values.
    count: u32,
}

impl MeanAccumulator {
    /// Folds one measured value in.
    fn add(&mut self, value: f64) {
        self.sum += value;
        self.count = self.count.saturating_add(1);
    }

    /// Mean over the measured values; 0.0 when nothing was measurable,
    /// matching the embeddings-off rendering convention.
    fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / f64::from(self.count)
    }
}
