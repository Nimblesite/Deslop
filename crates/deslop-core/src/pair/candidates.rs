//! Candidate pair construction, streamed and admission-gated
//! ([FUSION-STRATEGY-BOUNDED-MAX], [PERF-FLUTTER-TODO-PAIRS]).
//!
//! The historical construction materialised every unique pair the three
//! discovery sources surface — on the Flutter corpus, ~50 million
//! `CandidatePair` values plus the score maps that produced them,
//! gigabytes held simultaneously — and only then handed the whole list to
//! `cluster_by_transitive_closure`, which dropped almost all of it. The
//! construction below applies the same survival decision at insertion
//! time ([`super::construction_survives`], the identical arithmetic the
//! closure runs, evaluated with the shared-subtree overlap still
//! unknown): a pair that would be dropped, and cannot be rescued, never
//! enters the retained set. A pair that *can* be rescued is retained for
//! the measurement pass, exactly as before.
//!
//! The retained population is what the memory budget scales with now —
//! not the raw LSH pair volume.

use std::{collections::HashMap, hash::BuildHasher};

use super::{CandidatePair, PairScore, CROSS_LANGUAGE_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT};
use crate::{
    embedding::EmbeddingPair,
    fingerprint::{ranges_overlap, Fingerprint},
    lsh::{estimate_jaccard, Signature, SignatureIndex, SignatureLookup},
    state::FileId,
};

use builder::PairBuilder;

/// The insertion-time admission builder ([PERF-FLUTTER-TODO-PAIRS]).
mod builder;

/// A source of LSH band-collision pairs. Abstract so the batch render can
/// stream straight out of the band sort ([`crate::lsh::for_each_band_collision`])
/// while tests and small corpora can pass a materialised list.
pub trait LshPairs {
    /// Invokes `emit` for every collision pair `(i, j)`, `i < j`, in a
    /// deterministic order.
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize));
}

impl LshPairs for [(usize, usize)] {
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize)) {
        for &(left, right) in self {
            emit(left, right);
        }
    }
}

impl LshPairs for Vec<(usize, usize)> {
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize)) {
        self.as_slice().for_each(emit);
    }
}

impl LshPairs for &[(usize, usize)] {
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize)) {
        (**self).for_each(emit);
    }
}

/// Returns candidate pairs unioning:
///
/// - every distinct pair inside each structural (Merkle) hash bucket
///   (`structural = 1.0`),
/// - every LSH band collision (`structural = 0.0`),
/// - every ANN top-k neighbour surfaced by the embedding pass (pair
///   enters with its `embedding_cos` populated),
///
/// each admitted through [`construction_survives`] before it is
/// retained. When `embedding_pairs` is empty (no provider or
/// `--embeddings=off`) the surviving set matches the pre-P5 behaviour
/// exactly.
#[must_use]
pub fn candidate_pairs(
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    lsh_pairs: &dyn LshPairs,
    embedding_pairs: &[EmbeddingPair],
) -> Vec<CandidatePair> {
    build_candidates::<std::hash::RandomState>(
        fingerprints,
        signatures,
        lsh_pairs,
        embedding_pairs,
        None,
        false,
    )
}

/// Returns candidate pairs under the [CONFIG-CROSS-LANGUAGE] policy,
/// streamed and gated as [`candidate_pairs`] documents. The explicit
/// cross-language audit path (`allow_cross_language`) additionally
/// compares the cross-language signature space directly.
#[must_use]
pub fn candidate_pairs_for_language_policy<S: BuildHasher>(
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    lsh_pairs: &dyn LshPairs,
    embedding_pairs: &[EmbeddingPair],
    cross_language_signatures: Option<&[Signature]>,
    file_languages: &HashMap<FileId, &'static str, S>,
    allow_cross_language: bool,
) -> Vec<CandidatePair> {
    let mut pairs = build_candidates(
        fingerprints,
        signatures,
        lsh_pairs,
        embedding_pairs,
        Some(file_languages),
        allow_cross_language,
    );
    if allow_cross_language {
        // The audit space is the explicit cross-language signature list
        // when the pass built one, else the per-language space itself.
        let built;
        let alias_space: &dyn SignatureLookup = match cross_language_signatures {
            Some(space) => {
                built = SignatureIndex::from_segments([space]);
                &built
            }
            None => signatures,
        };
        add_cross_language_signature_pairs(&mut pairs, fingerprints, alias_space, file_languages);
    }
    pairs
}

/// The streamed, gated construction core shared by both entry points.
/// `file_languages == None` skips the language policy entirely.
fn build_candidates<S: BuildHasher>(
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    lsh_pairs: &dyn LshPairs,
    embedding_pairs: &[EmbeddingPair],
    file_languages: Option<&HashMap<FileId, &'static str, S>>,
    allow_cross_language: bool,
) -> Vec<CandidatePair> {
    let mut builder = PairBuilder::new(
        fingerprints,
        signatures,
        file_languages,
        allow_cross_language,
    );
    tracing::info!(
        rss_mib = crate::observe::resident_mib(),
        "pairs: pre-structural"
    );
    builder.add_structural_pairs();
    tracing::info!(
        rss_mib = crate::observe::resident_mib(),
        evidence = builder.evidence.len(),
        "pairs: post-structural"
    );
    builder.merge_embedding_pairs(embedding_pairs);
    builder.flush_evidence();
    let mut lsh_scanned = 0_u64;
    lsh_pairs.for_each(&mut |left, right| {
        lsh_scanned = lsh_scanned.saturating_add(1);
        builder.add_zero_evidence(left, right);
    });
    tracing::info!(
        rss_mib = crate::observe::resident_mib(),
        kept = builder.kept.len(),
        lsh_scanned,
        "pairs: post-lsh"
    );
    let resolved = builder.finish();
    tracing::info!(
        rss_mib = crate::observe::resident_mib(),
        resolved = resolved.len(),
        "pairs: post-resolve"
    );
    resolved
}

/// Adds direct signature matches for explicit cross-language audits —
/// the opt-in O(n²) comparison space, gated like every other source.
fn add_cross_language_signature_pairs<S: BuildHasher>(
    pairs: &mut Vec<CandidatePair>,
    fingerprints: &[Fingerprint],
    signatures: &dyn SignatureLookup,
    file_languages: &HashMap<FileId, &'static str, S>,
) {
    let existing: std::collections::BTreeSet<(usize, usize)> = pairs.iter().map(pair_key).collect();
    let limit = fingerprints.len().min(signatures.len());
    let mut additions = Vec::new();
    for left in 0..limit {
        for right in (left.saturating_add(1))..limit {
            let key = order(left, right);
            if existing.contains(&key)
                || same_language_indexes(left, right, fingerprints, file_languages)
            {
                continue;
            }
            let token_jaccard = jaccard_for(signatures, left, right);
            if token_jaccard < CROSS_LANGUAGE_MIN_JACCARD {
                continue;
            }
            let endpoint_node_counts = endpoint_node_counts(fingerprints, left, right);
            additions.push(CandidatePair {
                left,
                right,
                endpoint_node_counts,
                lsh_only_node_floor: endpoint_node_counts.0.max(LSH_ONLY_MIN_NODE_COUNT),
                lsh_only_min_jaccard: CROSS_LANGUAGE_MIN_JACCARD,
                fused_min_score: CROSS_LANGUAGE_MIN_JACCARD,
                shared_subtree_overlap: 0.0,
                score: PairScore {
                    structural: 0.0,
                    token_jaccard,
                    embedding_cos: 0.0,
                },
            });
        }
    }
    pairs.extend(additions);
    pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
    pairs.dedup_by_key(|pair| (pair.left, pair.right));
}

/// A pair's order-insensitive key.
fn pair_key(pair: &CandidatePair) -> (usize, usize) {
    order(pair.left, pair.right)
}

/// True when both fingerprint indexes resolve to the same language id.
fn same_language_indexes<S: BuildHasher>(
    left_index: usize,
    right_index: usize,
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let Some(left) = fingerprints.get(left_index) else {
        return false;
    };
    let Some(right) = fingerprints.get(right_index) else {
        return false;
    };
    match (
        file_languages.get(&left.file_id),
        file_languages.get(&right.file_id),
    ) {
        (Some(left_language), Some(right_language)) => left_language == right_language,
        _ => false,
    }
}

/// True when the pair's endpoints live in different files — the rescue
/// route's scope ([FUSION-SHARED-SUBTREE]).
fn pair_crosses_files(pair: &CandidatePair, fingerprints: &[Fingerprint]) -> bool {
    match (fingerprints.get(pair.left), fingerprints.get(pair.right)) {
        (Some(left), Some(right)) => left.file_id != right.file_id,
        _ => false,
    }
}

/// Keeps non-structural candidates from connecting nested same-file ranges.
fn candidate_ranges_are_valid(pair: &CandidatePair, fingerprints: &[Fingerprint]) -> bool {
    if pair.score.structural > 0.0 {
        return true;
    }
    let Some(left) = fingerprints.get(pair.left) else {
        return false;
    };
    let Some(right) = fingerprints.get(pair.right) else {
        return false;
    };
    left.file_id != right.file_id || !ranges_overlap(left, right)
}

/// Returns both endpoint node counts as `(smaller, larger)`. Defaults a
/// missing endpoint to 0 — an impossible state in the current pipeline,
/// but keeps the helper total.
fn endpoint_node_counts(fingerprints: &[Fingerprint], left: usize, right: usize) -> (usize, usize) {
    let left_count = fingerprints
        .get(left)
        .map_or(0, |fingerprint| fingerprint.node_count);
    let right_count = fingerprints
        .get(right)
        .map_or(0, |fingerprint| fingerprint.node_count);
    (left_count.min(right_count), left_count.max(right_count))
}

/// Looks up both signatures and returns their estimated Jaccard. Returns
/// 0.0 when either signature is missing, which cannot happen in practice
/// because the pipeline always produces one signature per fingerprint.
fn jaccard_for(signatures: &dyn SignatureLookup, left: usize, right: usize) -> f64 {
    match (signatures.signature(left), signatures.signature(right)) {
        (Some(left_signature), Some(right_signature)) => {
            estimate_jaccard(left_signature, right_signature)
        }
        _ => 0.0,
    }
}

/// Puts the smaller index first. Pair keys are order-insensitive.
fn order(left: usize, right: usize) -> (usize, usize) {
    (left.min(right), left.max(right))
}
