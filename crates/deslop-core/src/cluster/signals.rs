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
    lsh::{estimate_jaccard, Signature, SignatureLookup},
    overlap::OverlapMeasurer,
    pair::PairScore,
};

/// Measures the [FUSION-CLUSTER-SIGNALS] triple over the rendered
/// occurrence indices.
///
/// Per occurrence pair: `structural` is Merkle-hash equality — `1.0` —
/// or, for a non-equal pair, the measured shared-subtree overlap
/// ([FUSION-SHARED-SUBTREE]): the best-achievable subtree overlap the
/// axis has always claimed to be. `token_jaccard` is the `MinHash`
/// estimate between the two signatures, and `embedding_cos` is
/// [`cosine_similarity`] of the two vectors — the same arithmetic that
/// admitted the ANN pair evidence. A pair missing an input for a
/// signal (no vector: embeddings off, oversized input, provider
/// failure) is excluded from that signal's numerator and denominator
/// both, so absence never masquerades as a measured 0.0 inside the
/// mean.
///
/// The left occurrence is resolved once per row rather than once per
/// pair ([PERF-FLUTTER-TODO-PAIRS]). A wide cluster measures hundreds
/// of thousands of pairs against the same left side, and each
/// resolution is a segment binary search plus a hash lookup; the pairs
/// measured and the order they fold in are unchanged.
pub(super) fn measured_signals<S: BuildHasher>(
    occurrence_indices: &[usize],
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    embedding_vectors: &HashMap<usize, Vec<f32>, S>,
    overlap: &mut OverlapMeasurer<'_>,
) -> PairScore {
    let corpus = SignalCorpus {
        fingerprints,
        signatures,
        embedding_vectors,
    };
    let mut totals = SignalTotals::default();
    for (position, &left) in occurrence_indices.iter().enumerate() {
        let left_side = corpus.side(left);
        for &right in occurrence_indices.iter().skip(position.saturating_add(1)) {
            totals.add_pair(left_side, corpus.side(right), overlap);
        }
    }
    totals.means()
}

/// The corpus-wide sources one occurrence's signal inputs resolve from.
struct SignalCorpus<'corpus, S> {
    /// Every fingerprint, indexed by occurrence index.
    fingerprints: &'corpus [Fingerprint],
    /// The `MinHash` signature population.
    signatures: &'corpus dyn SignatureLookup,
    /// Embedding vectors by occurrence index, empty when embeddings are
    /// off.
    embedding_vectors: &'corpus HashMap<usize, Vec<f32>, S>,
}

impl<S: BuildHasher> SignalCorpus<'_, S> {
    /// Resolves the three signal inputs for one occurrence. Each is
    /// independently optional: a signal is measured only for the pairs
    /// that have both of its inputs.
    fn side(&self, index: usize) -> SignalSide<'_> {
        SignalSide {
            fingerprint: self.fingerprints.get(index),
            signature: self.signatures.signature(index),
            vector: self.embedding_vectors.get(&index),
        }
    }
}

/// One occurrence's borrowed signal inputs. Borrowed rather than
/// copied: a [`Signature`] is a kilobyte, and copying one per pair cost
/// the Flutter corpus 32 GB of memcpy in this stage alone.
#[derive(Clone, Copy)]
struct SignalSide<'corpus> {
    /// The occurrence's fingerprint, for the structural axis.
    fingerprint: Option<&'corpus Fingerprint>,
    /// Its `MinHash` signature, for the token axis.
    signature: Option<&'corpus Signature>,
    /// Its embedding vector, for the cosine axis.
    vector: Option<&'corpus Vec<f32>>,
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
    fn add_pair(
        &mut self,
        left: SignalSide<'_>,
        right: SignalSide<'_>,
        overlap: &mut OverlapMeasurer<'_>,
    ) {
        if let (Some(left_fp), Some(right_fp)) = (left.fingerprint, right.fingerprint) {
            // Merkle equality short-circuits to 1.0 inside
            // `OverlapMeasurer::overlap`; a non-equal pair measures its
            // shared-subtree overlap ([FUSION-SHARED-SUBTREE]).
            self.structural.add(overlap.overlap(left_fp, right_fp));
        }
        if let (Some(left_sig), Some(right_sig)) = (left.signature, right.signature) {
            self.token_jaccard
                .add(estimate_jaccard(left_sig, right_sig));
        }
        if let (Some(left_vec), Some(right_vec)) = (left.vector, right.vector) {
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
