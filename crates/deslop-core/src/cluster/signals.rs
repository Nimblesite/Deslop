//! Rendered-truth cluster signal measurement ([FUSION-CLUSTER-SIGNALS]).
//!
//! A cluster's signal triple is measured **between the admitted pairs** —
//! the surviving discovery edges of the transitive-closure component —
//! never between every unordered pair of rendered occurrences. Each axis
//! reports the strongest measurement any admitted pair earned on that
//! axis, and a pair that never cleared `admission.fused_threshold`
//! contributes nothing (gh #458). Averaging is wrong on both counts:
//! Baker (1995) defines duplication as a per-pair predicate with no
//! class-level average to take, and averaging closure-only pairs lets
//! one unrelated member dilute N proven copies — a byte-identical file
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

/// The measured rendered signal triple plus the pair that earned it.
///
/// [FUSION-CLUSTER-SIGNALS] gh #458: the rendered triple is one
/// admitted pair's own measurement — the strongest one, in a
/// deterministic order — so the report can name the pair whose
/// evidence it shows instead of an anonymous cluster average and
/// instead of a per-axis stitch that no single pair ever earned.
pub(super) struct MeasuredSignals {
    /// The rendered triple: the elected admitted pair's own axes
    /// together, never assembled from different pairs (a pair whose
    /// signal input is absent renders 0.0 on that axis).
    pub score: PairScore,
    /// The admitted pair (corpus indices) whose measurement the
    /// rendered triple displays, in a deterministic order; `None` when
    /// no admitted pair survives the same-file collapse.
    pub source_pair: Option<(usize, usize)>,
}

/// Measures the [FUSION-CLUSTER-SIGNALS] triple over the **admitted**
/// pair set: the pair with the highest bounded-fused confidence,
/// ties resolved to the earliest pair in corpus order.
///
/// Per admitted pair: `structural` is Merkle-hash equality — `1.0` —
/// or, for a non-equal pair, the measured shared-subtree overlap
/// ([FUSION-SHARED-SUBTREE]): the best-achievable subtree overlap the
/// axis has always claimed to be. `token_jaccard` is the `MinHash`
/// estimate between the two signatures, and `embedding_cos` is
/// [`cosine_similarity`] of the two vectors — the same arithmetic that
/// admitted the pair evidence. A pair missing an input for a signal
/// (no vector: embeddings off, oversized input, provider failure)
/// renders 0.0 on that axis, so absence never masquerades as a
/// measured value.
///
/// The left occurrence is resolved once per row rather than once per
/// pair ([PERF-FLUTTER-TODO-PAIRS]). A wide cluster measures hundreds
/// of thousands of admitted pairs against the same left side, and each
/// resolution is a segment binary search plus a hash lookup; the pairs
/// measured and the order they fold in are unchanged.
pub(super) fn measured_signals<S: BuildHasher>(
    occurrence_indices: &[usize],
    admitted_pairs: &[(usize, usize)],
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    embedding_vectors: &HashMap<usize, Vec<f32>, S>,
    overlap: &mut OverlapMeasurer<'_>,
) -> MeasuredSignals {
    let corpus = SignalCorpus {
        fingerprints,
        signatures,
        embedding_vectors,
    };
    let (sides, mut values) = grouped_sides(&corpus, occurrence_indices, overlap);
    let positions: HashMap<usize, usize> = occurrence_indices
        .iter()
        .enumerate()
        .map(|(position, &index)| (index, position))
        .collect();
    let mut best: Option<BestPair> = None;
    fold_pairs(
        &sides,
        &positions,
        admitted_pairs,
        &mut values,
        &mut best,
        overlap,
    );
    MeasuredSignals {
        // A cluster whose every admitted pair skipped renders an
        // explicit 0.0 triple with no named source — never a silent
        // empty ([FUSION-CLUSTER-SIGNALS] gh #458).
        score: best.map_or(
            PairScore {
                structural: 0.0,
                token_jaccard: 0.0,
                embedding_cos: 0.0,
            },
            BestPair::score,
        ),
        source_pair: best.map(|pair| (pair.left, pair.right)),
    }
}

/// Folds only the admitted pairs, in their deterministic edge order.
///
/// A pair whose endpoint was collapsed away by the same-file overlap
/// collapse is skipped: its evidence described within-file duplication,
/// which never needs to survive a same-file collapse to stay reported
/// (#339). When every admitted pair skips, the cluster renders a
/// 0.0 triple with no named source — the `None` case is explicit, never
/// a silent empty.
fn fold_pairs(
    sides: &[GroupedSignalSide<'_>],
    positions: &HashMap<usize, usize>,
    admitted_pairs: &[(usize, usize)],
    values: &mut SignalValues,
    best: &mut Option<BestPair>,
    overlap: &mut OverlapMeasurer<'_>,
) {
    for &(left, right) in admitted_pairs {
        let (Some(&left_position), Some(&right_position)) =
            (positions.get(&left), positions.get(&right))
        else {
            continue;
        };
        let pair = BestPair::measure(
            left,
            right,
            sides[left_position],
            sides[right_position],
            values,
            overlap,
        );
        if best.as_ref().is_none_or(|current| pair.stronger_than(current)) {
            *best = Some(pair);
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

/// One admitted pair's measured triple, and the deterministic ordering
/// that elects the named signal source ([FUSION-CLUSTER-SIGNALS]). The
/// elected pair's axes render together; the other admitted pairs only
/// contest the election.
#[derive(Debug, Clone, Copy)]
struct BestPair {
    /// Lower corpus index of the pair.
    left: usize,
    /// Higher corpus index of the pair.
    right: usize,
    /// Measured structural value, when the pair has the input.
    structural: Option<f64>,
    /// Measured token-Jaccard value, when the pair has the input.
    token_jaccard: Option<f64>,
    /// Measured embedding cosine, when the pair has the input.
    embedding_cos: Option<f64>,
}

impl BestPair {
    /// Measures one admitted pair's triple from its sides.
    fn measure(
        left: usize,
        right: usize,
        left_side: GroupedSignalSide<'_>,
        right_side: GroupedSignalSide<'_>,
        values: &mut SignalValues,
        overlap: &mut OverlapMeasurer<'_>,
    ) -> Self {
        Self {
            left,
            right,
            structural: values.structural(left_side, right_side, overlap),
            token_jaccard: values.token(left_side, right_side),
            embedding_cos: left_side
                .inputs
                .vector
                .zip(right_side.inputs.vector)
                .map(|(left_vec, right_vec)| cosine_similarity(left_vec, right_vec)),
        }
    }

    /// The rendered score: this pair's own triple together, with an
    /// absent signal input rendered 0.0 — the embeddings-off rendering
    /// convention. Never assembled from different pairs.
    fn score(self) -> PairScore {
        PairScore {
            structural: self.structural.unwrap_or(0.0),
            token_jaccard: self.token_jaccard.unwrap_or(0.0),
            embedding_cos: self.embedding_cos.unwrap_or(0.0),
        }
    }

    /// Whether this pair beats `current` in the deterministic
    /// strongest-pair order: bounded-fused confidence first, ties
    /// resolved to the earliest pair in corpus order (#301), so a
    /// byte-identical pair (`90, 360`) is named over a same-shape pair
    /// (`90, 630`) that measures identically after normalisation. The
    /// fused confidence is the pair's own bounded max
    /// ([FUSION-STRATEGY-BOUNDED-MAX]).
    fn stronger_than(self, current: &BestPair) -> bool {
        let rank = |pair: &BestPair| {
            (
                pair.fused(),
                std::cmp::Reverse(pair.left),
                std::cmp::Reverse(pair.right),
            )
        };
        rank(&self) > rank(current)
    }

    /// The pair's bounded-fused confidence: the strongest single axis,
    /// clamped to `[0, 1]`.
    fn fused(self) -> f64 {
        self.structural
            .unwrap_or(0.0)
            .max(self.token_jaccard.unwrap_or(0.0))
            .max(self.embedding_cos.unwrap_or(0.0))
            .clamp(0.0, 1.0)
    }
}
