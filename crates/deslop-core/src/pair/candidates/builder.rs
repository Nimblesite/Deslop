//! Insertion-time candidate admission ([PERF-FLUTTER-TODO-PAIRS],
//! [PERF-FLUTTER-TODO-MEMORY]): the [`PairBuilder`] that streams the
//! three discovery sources through the survival gate so the retained
//! pair set — not the raw LSH volume — is what memory scales with.
//! Split from the parent module, which owns the entry points and the
//! shared endpoint helpers.

use std::{collections::HashMap, hash::BuildHasher};

use super::super::{
    construction_survives, rescue_eligible, CandidatePair, PairScore, CROSS_LANGUAGE_MIN_JACCARD,
    FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT,
};
use super::{
    candidate_ranges_are_valid, endpoint_node_counts, jaccard_for, order, pair_crosses_files,
    same_language_indexes,
};
use crate::{
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::SignatureLookup,
    state::FileId,
};

/// The ordered pair key packed as one `u64`: high half the lower
/// index, low half the higher ([PERF-FLUTTER-TODO-MEMORY]).
fn packed_key(key: (usize, usize)) -> u64 {
    let (left, right) = key;
    (u64::try_from(left).unwrap_or(u64::MAX) << 32)
        | u64::try_from(right).unwrap_or(0xFFFF_FFFF)
}

/// The inverse of [`packed_key`]: the ordered index pair a row's packed
/// key carries.
fn unpack_key(key: u64) -> (usize, usize) {
    let high = usize::try_from(key >> 32).unwrap_or(usize::MAX);
    let low = usize::try_from(key & 0xFFFF_FFFF).unwrap_or(usize::MAX);
    (high, low)
}

/// Insertion-time admission state: the retained pair list plus the
/// lookups a pair's verdict needs.
///
/// The retained population is a flat `Vec` — no payload map, no
/// per-insert membership test: on a corpus-scale run (4M+ retained
/// pairs) a `HashMap<(usize, usize), CandidatePair>` costs several GB
/// once entry slack and rehash doubling are counted. Duplicate keys
/// (several bands, or the structural/embedding passes, can emit one
/// pair) are resolved once after collection
/// ([PERF-FLUTTER-TODO-MEMORY]).
pub(super) struct PairBuilder<'corpus, S: BuildHasher> {
    /// Fingerprints the pair endpoints index into.
    fingerprints: &'corpus [Fingerprint],
    /// Signatures for the token-Jaccard axis.
    signatures: &'corpus dyn SignatureLookup,
    /// Language policy lookup; `None` admits every language pair.
    file_languages: Option<&'corpus HashMap<FileId, &'static str, S>>,
    /// Whether explicit cross-language comparison is allowed.
    allow_cross_language: bool,
    /// The gated, retained pairs ([PERF-FLUTTER-TODO-MEMORY]).
    pub(super) kept: Vec<CandidatePair>,
    /// Merged evidence for every evidence-bearing key
    /// ([REPAIR-COSINE-MERGE], gh #351): the structural axis from the
    /// Merkle pass and the strongest cosine from the embedding pass,
    /// per axis — whichever pass reached the pair first is telemetry.
    /// The LSH bulk carries no evidence and arrives after both passes,
    /// so it never needs a row here
    /// (`docs/performance-branch-review.md`, "first-seen pair
    /// deduplication drops stronger evidence").
    pub(super) evidence: HashMap<u64, (f64, f64)>,
    /// Packed keys already carried by `kept` — refuses the re-emission
    /// a pair suffers every time it collides in another band (a
    /// retained pair averages a dozen emissions on a corpus-scale run
    /// — pushing every one again is gigabytes of dead entries).
    kept_keys: std::collections::HashSet<u64>,
}

impl<'corpus, S: BuildHasher> PairBuilder<'corpus, S> {
    /// Builder over one corpus view.
    pub(super) fn new(
        fingerprints: &'corpus [Fingerprint],
        signatures: &'corpus dyn SignatureLookup,
        file_languages: Option<&'corpus HashMap<FileId, &'static str, S>>,
        allow_cross_language: bool,
    ) -> Self {
        Self {
            fingerprints,
            signatures,
            file_languages,
            allow_cross_language,
            kept: Vec::new(),
            evidence: HashMap::new(),
            kept_keys: std::collections::HashSet::new(),
        }
    }

    /// Adds the structural (Merkle) star pairs: the canonical member of
    /// each bucket paired with every other member — `O(n log n)` over
    /// one flat `(hash, index)` array, the same topology the LSH pass
    /// uses.
    ///
    /// The historical `HashMap<hash, Vec<index>>` populated one tiny
    /// heap vector per distinct hash — millions of small allocations on
    /// a corpus-scale run, which the allocator strands in per-size
    /// arenas long after the map dies. One sortable array is a single
    /// large allocation the allocator returns whole
    /// ([PERF-FLUTTER-TODO-MEMORY]); the emitted pair set is identical
    /// because each bucket's star is (minimum index, each other) in
    /// index order either way.
    pub(super) fn add_structural_pairs(&mut self) {
        let mut tagged: Vec<([u8; 32], usize)> = self
            .fingerprints
            .iter()
            .enumerate()
            .map(|(index, fingerprint)| (fingerprint.hash, index))
            .collect();
        tagged.sort_unstable();
        let mut run_start = 0_usize;
        while let Some((run_hash, _)) = tagged.get(run_start).copied() {
            let run_end = tagged
                .get(run_start.saturating_add(1)..)
                .unwrap_or(&[])
                .iter()
                .position(|(hash, _)| *hash != run_hash)
                .map_or(tagged.len(), |offset| run_start.saturating_add(1).saturating_add(offset));
            let run = tagged.get(run_start..run_end).unwrap_or(&[]);
            let Some(canonical) = run.first().map(|(_, index)| *index) else {
                break;
            };
            for (_, other) in run.iter().skip(1) {
                self.add_evidence(canonical, *other, 1.0, 0.0);
            }
            run_start = run_end;
        }
    }

    /// Merges the embedding ANN pairs, recording each measured cosine
    /// into the pair whether or not an earlier pass surfaced it
    /// ([REPAIR-COSINE-MERGE], gh #351): a cosine is evidence about the
    /// pair, and the pass that reached it first is telemetry. The
    /// insertion-time gate sees the cosine — an embedding-discovered
    /// pair is admitted on its own evidence, not on a stub.
    pub(super) fn merge_embedding_pairs(&mut self, embedding_pairs: &[EmbeddingPair]) {
        for pair in embedding_pairs {
            self.add_evidence(pair.left, pair.right, 0.0, pair.cosine);
        }
    }

    /// Finishes the build: releases the key set and returns the kept
    /// pairs in deterministic key order.
    pub(super) fn finish(mut self) -> Vec<CandidatePair> {
        drop(std::mem::take(&mut self.kept_keys));
        self.kept.sort_unstable_by_key(|pair| (pair.left, pair.right));
        self.kept.shrink_to_fit();
        self.kept
    }

    /// Records evidence-bearing discovery of `(left, right)`: the
    /// structural axis and the cosine merge per axis into the key's
    /// row, so no arrival order can drop the stronger evidence.
    fn add_evidence(&mut self, left: usize, right: usize, structural: f64, cosine: f64) {
        let key = packed_key(order(left, right));
        let row = self.evidence.entry(key).or_insert((0.0, 0.0));
        row.0 = row.0.max(structural);
        row.1 = row.1.max(cosine);
    }

    /// Constructs, gates, and retains every evidence-bearing key on its
    /// merged evidence, then records the key as carried — including
    /// refused ones, whose zero-evidence re-discovery is monotone
    /// weaker and can never change the verdict.
    pub(super) fn flush_evidence(&mut self) {
        for (key, (structural, cosine)) in std::mem::take(&mut self.evidence) {
            let _carried = self.kept_keys.insert(key);
            let Some(pair) = self.construct_pair(unpack_key(key), structural, cosine) else {
                continue;
            };
            if self.gate(&pair) {
                self.kept.push(pair);
            }
        }
    }

    /// A zero-evidence (LSH band-collision) discovery: construct, gate,
    /// and retain only if the key is not already carried.
    pub(super) fn add_zero_evidence(&mut self, left: usize, right: usize) {
        let key = packed_key(order(left, right));
        if !self.kept_keys.insert(key) {
            return;
        }
        let Some(pair) = self.construct_pair((left, right), 0.0, 0.0) else {
            return;
        };
        if self.gate(&pair) {
            self.kept.push(pair);
        }
    }

    /// Builds the candidate for `key` with its score axes filled in, or
    /// `None` when the language policy excludes the pair outright.
    fn construct_pair(
        &self,
        key: (usize, usize),
        structural: f64,
        cosine: f64,
    ) -> Option<CandidatePair> {
        let (left, right) = key;
        let endpoint_node_counts = endpoint_node_counts(self.fingerprints, left, right);
        let mut pair = CandidatePair {
            left,
            right,
            endpoint_node_counts,
            lsh_only_node_floor: endpoint_node_counts.0,
            lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
            fused_min_score: FUSED_THRESHOLD,
            shared_subtree_overlap: 0.0,
            score: PairScore {
                structural,
                token_jaccard: jaccard_for(self.signatures, left, right),
                embedding_cos: cosine,
            },
        };
        self.apply_language_policy(&mut pair)?;
        Some(pair)
    }

    /// Applies [CONFIG-CROSS-LANGUAGE]: drops cross-language pairs when
    /// the audit mode is off; lowers the admission floors for them when
    /// it is on. Returns `None` when the pair is excluded. No language
    /// map means no policy — every pair passes through untouched.
    fn apply_language_policy(&self, pair: &mut CandidatePair) -> Option<()> {
        let Some(languages) = self.file_languages else {
            return Some(());
        };
        if !same_language_indexes(pair.left, pair.right, self.fingerprints, languages)
            && !self.allow_cross_language
        {
            return None;
        }
        if pair.score.structural <= 0.0
            && !same_language_indexes(pair.left, pair.right, self.fingerprints, languages)
        {
            pair.lsh_only_node_floor = pair.lsh_only_node_floor.max(LSH_ONLY_MIN_NODE_COUNT);
            pair.lsh_only_min_jaccard = CROSS_LANGUAGE_MIN_JACCARD;
            pair.fused_min_score = CROSS_LANGUAGE_MIN_JACCARD;
        }
        Some(())
    }

    /// The insertion-time admission decision
    /// ([PERF-FLUTTER-TODO-PAIRS]): keep a pair that survives with the
    /// shared-subtree overlap unknown, or one the rescue could still
    /// admit — those are the pairs the closure would keep, so the
    /// retained set induces the same clusters the ungated construction
    /// produced, at a fraction of the resident memory.
    fn gate(&self, pair: &CandidatePair) -> bool {
        if !candidate_ranges_are_valid(pair, self.fingerprints) {
            return false;
        }
        construction_survives(pair)
            || (rescue_eligible(pair) && pair_crosses_files(pair, self.fingerprints))
    }
}
