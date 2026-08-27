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

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use crate::{
    embedding::cosine_similarity,
    fingerprint::Fingerprint,
    lsh::{estimate_jaccard, Signature, SignatureLookup},
    overlap::OverlapMeasurer,
    pair::PairScore,
};

#[cfg(test)]
mod tests;

/// Number of distinct values in the one-group fast path.
const SINGLE_GROUP_COUNT: usize = 1;

/// Compact pair key for that sole group.
const SINGLE_GROUP_PAIR: (usize, usize) = (0, 0);

/// Members required for two distinct same-group occurrence pairs.
const SAME_GROUP_CACHE_MIN_MEMBERS: usize = 3;

/// Members required for a cross-group pair value to repeat.
const CROSS_GROUP_CACHE_MIN_MEMBERS: usize = 2;

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
    let (sides, mut values) = grouped_sides(&corpus, occurrence_indices, overlap);
    let mut totals = SignalTotals::default();
    fold_pairs(&sides, &mut values, &mut totals, overlap);
    totals.means()
}

/// Folds cached valuations in the original occurrence-pair order.
fn fold_pairs(
    sides: &[GroupedSignalSide<'_>],
    values: &mut SignalValues,
    totals: &mut SignalTotals,
    overlap: &mut OverlapMeasurer<'_>,
) {
    for (position, &left) in sides.iter().enumerate() {
        for &right in sides.iter().skip(position.saturating_add(1)) {
            totals.add_pair(left, right, values, overlap);
        }
    }
}

/// Resolves occurrence inputs once and assigns compact identities to
/// equal structural and token values.
fn grouped_sides<'corpus, S: BuildHasher>(
    corpus: &SignalCorpus<'corpus, S>,
    occurrence_indices: &[usize],
    overlap: &mut OverlapMeasurer<'_>,
) -> (Vec<GroupedSignalSide<'corpus>>, SignalValues) {
    let resolve = corpus.has_multiple_structural_hashes(occurrence_indices);
    let mut groups = SignalGroups::new();
    let sides = occurrence_indices
        .iter()
        .map(|&index| groups.add(corpus.side(index), resolve, overlap))
        .collect();
    (sides, groups.into_values())
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

impl<'corpus, S: BuildHasher> SignalCorpus<'corpus, S> {
    /// Resolves the three signal inputs for one occurrence. Each is
    /// independently optional: a signal is measured only for the pairs
    /// that have both of its inputs.
    fn side(&self, index: usize) -> SignalSide<'corpus> {
        SignalSide {
            fingerprint: self.fingerprints.get(index),
            signature: self.signatures.signature(index),
            vector: self.embedding_vectors.get(&index),
        }
    }

    /// Whether rendered occurrences contain more than one Merkle hash.
    /// A one-hash cluster needs no endpoint resolution: every structural
    /// pair returns `1.0` before consulting the trees.
    fn has_multiple_structural_hashes(&self, indices: &[usize]) -> bool {
        let mut hashes = indices
            .iter()
            .filter_map(|&index| self.fingerprints.get(index))
            .map(|fingerprint| fingerprint.hash);
        let Some(first) = hashes.next() else {
            return false;
        };
        hashes.any(|hash| hash != first)
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

/// One side plus its compact cache identities.
#[derive(Clone, Copy)]
struct GroupedSignalSide<'corpus> {
    /// Raw signal inputs.
    inputs: SignalSide<'corpus>,
    /// Structural group, partitioned by hash and resolvability.
    structural_group: Option<usize>,
    /// Exact signature-content group.
    token_group: Option<usize>,
}

/// Merkle identity plus endpoint resolvability. Equal hashes always
/// score `1.0`, but unequal hashes with an unresolvable side score `0.0`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct StructuralGroup {
    /// Merkle hash.
    hash: [u8; 32],
    /// Whether the endpoint range resolves when multiple hashes exist.
    resolvable: bool,
}

/// Compact group ids and occurrence counts for one equality key.
struct GroupTable<K> {
    /// Equality key to compact id.
    ids: HashMap<K, usize>,
    /// Occurrences assigned to each compact id.
    sizes: Vec<usize>,
}

impl<K: Eq + Hash> GroupTable<K> {
    /// Empty group table.
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            sizes: Vec::new(),
        }
    }

    /// Returns the key's id and records one occurrence.
    fn add(&mut self, key: K) -> usize {
        if let Some(&id) = self.ids.get(&key) {
            self.bump(id);
            return id;
        }
        let id = self.sizes.len();
        self.sizes.push(1);
        let _previous = self.ids.insert(key, id);
        id
    }

    /// Increments an existing group's population without indexing.
    fn bump(&mut self, id: usize) {
        if let Some(size) = self.sizes.get_mut(id) {
            *size = size.saturating_add(1);
        }
    }
}

/// Builds structural and token group ids while sides resolve once.
struct SignalGroups<'corpus> {
    /// Structural equality groups.
    structural: GroupTable<StructuralGroup>,
    /// Token-signature equality groups.
    token: GroupTable<&'corpus Signature>,
}

impl<'corpus> SignalGroups<'corpus> {
    /// Empty signal grouping state.
    fn new() -> Self {
        Self {
            structural: GroupTable::new(),
            token: GroupTable::new(),
        }
    }

    /// Assigns cache identities to one occurrence's available signals.
    fn add(
        &mut self,
        inputs: SignalSide<'corpus>,
        resolve: bool,
        overlap: &mut OverlapMeasurer<'_>,
    ) -> GroupedSignalSide<'corpus> {
        GroupedSignalSide {
            structural_group: inputs
                .fingerprint
                .map(|fingerprint| self.structural_id(fingerprint, resolve, overlap)),
            token_group: inputs.signature.map(|signature| self.token.add(signature)),
            inputs,
        }
    }

    /// Structural id, resolving only when unequal hashes make it matter.
    fn structural_id(
        &mut self,
        fingerprint: &Fingerprint,
        resolve: bool,
        overlap: &mut OverlapMeasurer<'_>,
    ) -> usize {
        self.structural.add(StructuralGroup {
            hash: fingerprint.hash,
            resolvable: !resolve || overlap.resolvable(fingerprint),
        })
    }

    /// Converts group populations into value caches.
    fn into_values(self) -> SignalValues {
        SignalValues {
            structural: PairValueCache::new(self.structural.sizes),
            token: PairValueCache::new(self.token.sizes),
        }
    }
}

/// Cached expensive values for repeated group pairs.
struct SignalValues {
    /// Structural overlap by repeated group pair.
    structural: PairValueCache,
    /// `MinHash` estimate by repeated group pair.
    token: PairValueCache,
}

impl SignalValues {
    /// Structural value for one occurrence pair.
    fn structural(
        &mut self,
        left: GroupedSignalSide<'_>,
        right: GroupedSignalSide<'_>,
        overlap: &mut OverlapMeasurer<'_>,
    ) -> Option<f64> {
        let groups = left.structural_group.zip(right.structural_group)?;
        let inputs = left.inputs.fingerprint.zip(right.inputs.fingerprint)?;
        Some(
            self.structural
                .value(groups, || overlap.overlap(inputs.0, inputs.1)),
        )
    }

    /// Token-Jaccard value for one occurrence pair.
    fn token(&mut self, left: GroupedSignalSide<'_>, right: GroupedSignalSide<'_>) -> Option<f64> {
        let groups = left.token_group.zip(right.token_group)?;
        let inputs = left.inputs.signature.zip(right.inputs.signature)?;
        Some(
            self.token
                .value(groups, || estimate_jaccard(inputs.0, inputs.1)),
        )
    }
}

/// Values retained only when a group pair occurs more than once.
struct PairValueCache {
    /// Population per compact group id.
    group_sizes: Vec<usize>,
    /// Value for the overwhelmingly common one-group cluster, avoiding
    /// a hash-table lookup for every logical pair.
    single_value: Option<f64>,
    /// Already measured repeated group pairs.
    values: HashMap<(usize, usize), f64>,
}

impl PairValueCache {
    /// Cache over one signal's group populations.
    fn new(group_sizes: Vec<usize>) -> Self {
        Self {
            group_sizes,
            single_value: None,
            values: HashMap::new(),
        }
    }

    /// Returns one value, retaining it only when another pair reuses it.
    fn value(&mut self, groups: (usize, usize), compute: impl FnOnce() -> f64) -> f64 {
        let key = ordered_group_pair(groups);
        if self.group_sizes.len() == SINGLE_GROUP_COUNT && key == SINGLE_GROUP_PAIR {
            return self.single(compute);
        }
        if !self.repeats(key) {
            return compute();
        }
        if let Some(&cached) = self.values.get(&key) {
            return cached;
        }
        let value = compute();
        let _previous = self.values.insert(key, value);
        value
    }

    /// Returns or initializes the sole group's cached value.
    fn single(&mut self, compute: impl FnOnce() -> f64) -> f64 {
        if let Some(cached) = self.single_value {
            return cached;
        }
        let value = compute();
        self.single_value = Some(value);
        value
    }

    /// Whether more than one occurrence pair shares this group pair.
    fn repeats(&self, (left, right): (usize, usize)) -> bool {
        let left_size = self.group_sizes.get(left).copied().unwrap_or(0);
        let right_size = self.group_sizes.get(right).copied().unwrap_or(0);
        if left == right {
            return left_size >= SAME_GROUP_CACHE_MIN_MEMBERS;
        }
        left_size >= CROSS_GROUP_CACHE_MIN_MEMBERS || right_size >= CROSS_GROUP_CACHE_MIN_MEMBERS
    }
}

/// Order-insensitive cache identity for two compact groups.
fn ordered_group_pair((left, right): (usize, usize)) -> (usize, usize) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
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
        left: GroupedSignalSide<'_>,
        right: GroupedSignalSide<'_>,
        values: &mut SignalValues,
        overlap: &mut OverlapMeasurer<'_>,
    ) {
        if let Some(value) = values.structural(left, right, overlap) {
            self.structural.add(value);
        }
        if let Some(value) = values.token(left, right) {
            self.token_jaccard.add(value);
        }
        if let (Some(left_vec), Some(right_vec)) = (left.inputs.vector, right.inputs.vector) {
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
