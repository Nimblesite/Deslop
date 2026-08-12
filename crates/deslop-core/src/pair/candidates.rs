use std::{
    collections::{BTreeSet, HashMap},
    hash::BuildHasher,
};

use super::{
    CandidatePair, PairScore, CROSS_LANGUAGE_MIN_JACCARD, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD,
    LSH_ONLY_MIN_NODE_COUNT,
};
use crate::{
    embedding::EmbeddingPair,
    fingerprint::{ranges_overlap, Fingerprint},
    lsh::{estimate_jaccard, Signature},
    state::FileId,
};

/// Returns candidate pairs unioning:
///
/// - every distinct pair inside each structural (Merkle) hash bucket
///   (`structural = 1.0`),
/// - every LSH band collision (`structural = 0.0`),
/// - every ANN top-k neighbour surfaced by the embedding pass (pair
///   enters with its `embedding_cos` populated).
///
/// Pair scores include the token Jaccard estimate regardless of how the
/// pair was discovered. When `embedding_pairs` is empty (no provider
/// or `--embeddings=off`) the output matches the pre-P5 behaviour
/// exactly.
#[must_use]
pub fn candidate_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    lsh_pairs: &[(usize, usize)],
    embedding_pairs: &[EmbeddingPair],
) -> Vec<CandidatePair> {
    let mut scores: HashMap<(usize, usize), f64> = HashMap::new();
    let mut cosines: HashMap<(usize, usize), f64> = HashMap::new();
    collect_structural_pairs(fingerprints, &mut scores);
    add_lsh_pairs(lsh_pairs, &mut scores);
    add_embedding_pairs(embedding_pairs, fingerprints, &mut scores, &mut cosines);
    finalise_pairs(fingerprints, signatures, scores, &cosines)
}

/// Returns candidate pairs, optionally dropping cross-language endpoints
/// before transitive closure per [CONFIG-CROSS-LANGUAGE].
#[must_use]
pub fn candidate_pairs_for_language_policy<S: BuildHasher>(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    lsh_pairs: &[(usize, usize)],
    embedding_pairs: &[EmbeddingPair],
    cross_language_signatures: Option<&[Signature]>,
    file_languages: &HashMap<FileId, &'static str, S>,
    allow_cross_language: bool,
) -> Vec<CandidatePair> {
    let mut pairs = candidate_pairs(fingerprints, signatures, lsh_pairs, embedding_pairs);
    if allow_cross_language {
        add_cross_language_signature_pairs(
            &mut pairs,
            fingerprints,
            cross_language_signatures.unwrap_or(signatures),
            file_languages,
        );
        pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
        return pairs
            .into_iter()
            .map(|pair| cross_language_opt_in_pair(pair, fingerprints, file_languages))
            .collect();
    }
    pairs
        .into_iter()
        .filter(|pair| same_language_pair(pair, fingerprints, file_languages))
        .collect()
}

/// Explicit cross-language opt-in keeps LSH candidates subject to the
/// Jaccard/fused gates, but not the same-language low-node-count guard.
fn cross_language_opt_in_pair<S: BuildHasher>(
    mut pair: CandidatePair,
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> CandidatePair {
    if pair.score.structural <= 0.0 && !same_language_pair(&pair, fingerprints, file_languages) {
        pair.min_node_count = pair.min_node_count.max(LSH_ONLY_MIN_NODE_COUNT);
        pair.lsh_only_min_jaccard = CROSS_LANGUAGE_MIN_JACCARD;
        pair.fused_min_score = CROSS_LANGUAGE_MIN_JACCARD;
    }
    pair
}

/// Adds direct signature matches for explicit cross-language audits.
fn add_cross_language_signature_pairs<S: BuildHasher>(
    pairs: &mut Vec<CandidatePair>,
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    file_languages: &HashMap<FileId, &'static str, S>,
) {
    let mut existing: BTreeSet<(usize, usize)> = pairs.iter().map(pair_key).collect();
    let limit = fingerprints.len().min(signatures.len());
    for left in 0..limit {
        add_cross_language_signature_pairs_for_left(
            pairs,
            &mut existing,
            fingerprints,
            signatures,
            file_languages,
            left,
            limit,
        );
    }
}

/// Adds direct cross-language signature matches for one left endpoint.
fn add_cross_language_signature_pairs_for_left<S: BuildHasher>(
    pairs: &mut Vec<CandidatePair>,
    existing: &mut BTreeSet<(usize, usize)>,
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    file_languages: &HashMap<FileId, &'static str, S>,
    left: usize,
    limit: usize,
) {
    for right in (left.saturating_add(1))..limit {
        maybe_add_cross_language_signature_pair(
            pairs,
            existing,
            fingerprints,
            signatures,
            file_languages,
            left,
            right,
        );
    }
}

/// Adds one direct cross-language signature pair when it is above threshold.
fn maybe_add_cross_language_signature_pair<S: BuildHasher>(
    pairs: &mut Vec<CandidatePair>,
    existing: &mut BTreeSet<(usize, usize)>,
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    file_languages: &HashMap<FileId, &'static str, S>,
    left: usize,
    right: usize,
) {
    let key = order(left, right);
    if existing.contains(&key) || same_language_indexes(left, right, fingerprints, file_languages) {
        return;
    }
    let Some(left_signature) = signatures.get(left) else {
        return;
    };
    let Some(right_signature) = signatures.get(right) else {
        return;
    };
    let token_jaccard = estimate_jaccard(left_signature, right_signature);
    if token_jaccard < CROSS_LANGUAGE_MIN_JACCARD {
        return;
    }
    pairs.push(cross_language_signature_pair(
        fingerprints,
        left,
        right,
        token_jaccard,
    ));
    let _inserted = existing.insert(key);
}

/// Builds an LSH-only candidate from direct cross-language signature evidence.
fn cross_language_signature_pair(
    fingerprints: &[Fingerprint],
    left: usize,
    right: usize,
    token_jaccard: f64,
) -> CandidatePair {
    CandidatePair {
        left,
        right,
        min_node_count: min_node_count(fingerprints, left, right).max(LSH_ONLY_MIN_NODE_COUNT),
        lsh_only_min_jaccard: CROSS_LANGUAGE_MIN_JACCARD,
        fused_min_score: CROSS_LANGUAGE_MIN_JACCARD,
        score: PairScore {
            structural: 0.0,
            token_jaccard,
            embedding_cos: 0.0,
        },
    }
}

/// Returns a pair's order-insensitive key.
fn pair_key(pair: &CandidatePair) -> (usize, usize) {
    order(pair.left, pair.right)
}

/// Returns true when both pair endpoints resolve to the same language id.
fn same_language_pair<S: BuildHasher>(
    pair: &CandidatePair,
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    same_language_indexes(pair.left, pair.right, fingerprints, file_languages)
}

/// Returns true when both fingerprint indexes resolve to the same language id.
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

/// Populates `scores` with `1.0` for every structural (Merkle-hash) pair.
///
/// Uses a **star topology** per bucket rather than a full N² enumeration:
/// the canonical member of the bucket is paired with every other member,
/// which is `O(n)` per bucket and still produces the same connected
/// component under transitive closure. For a bucket of `2_000` clones this
/// is `2_000` pairs instead of `2_000_000` — critical on large
/// generated-code corpora.
fn collect_structural_pairs(
    fingerprints: &[Fingerprint],
    scores: &mut HashMap<(usize, usize), f64>,
) {
    let mut by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        by_hash.entry(fingerprint.hash).or_default().push(index);
    }
    for bucket in by_hash.values() {
        collect_structural_bucket(bucket, scores);
    }
}

/// Adds structural pairs for one Merkle bucket.
fn collect_structural_bucket(bucket: &[usize], scores: &mut HashMap<(usize, usize), f64>) {
    let mut sorted = bucket.to_vec();
    sorted.sort_unstable();
    let Some(canonical) = sorted.first().copied() else {
        return;
    };
    for other in sorted.iter().skip(1) {
        let key = order(canonical, *other);
        let _previous = scores.insert(key, 1.0_f64);
    }
}

/// Adds LSH-only pairs. Pairs already present (from the structural pass)
/// keep their existing score — structural evidence dominates token
/// evidence when both fire.
fn add_lsh_pairs(lsh_pairs: &[(usize, usize)], scores: &mut HashMap<(usize, usize), f64>) {
    for &(a, b) in lsh_pairs {
        let key = order(a, b);
        let _previous = scores.entry(key).or_insert(0.0_f64);
    }
}

/// Adds embedding ANN pairs. Same-file non-structural pairs retain the
/// embedding cosine so incidental local token overlap cannot erase
/// semantic signal; fresh pairs enter with their cosine populated. Pairs
/// already discovered structurally or by LSH hit the quarantine in
/// [`add_embedding_pair`] — the old arms discarded their measured cosine.
fn add_embedding_pairs(
    embedding_pairs: &[EmbeddingPair],
    fingerprints: &[Fingerprint],
    scores: &mut HashMap<(usize, usize), f64>,
    cosines: &mut HashMap<(usize, usize), f64>,
) {
    for pair in embedding_pairs {
        add_embedding_pair(pair, fingerprints, scores, cosines);
    }
}

/// Merges one embedding pair into the candidate-score maps.
///
/// # QUARANTINED — measured cosines were silently discarded
///
/// The deleted arms threw away `pair.cosine` — a similarity the ANN pass
/// actually measured — whenever the pair was already known:
///
/// - `Some(structural) if structural > 0.0 => {}` dropped the cosine for
///   structurally-discovered pairs, so a byte-identical pair scanned with
///   `--embeddings required` rendered `embedding_cos = 0.0` — a false
///   reported figure indistinguishable from "semantically unrelated".
/// - `Some(_) => {}` dropped the cosine for cross-file pairs LSH also
///   surfaced. With the cosine zeroed, `survival_decision` classifies the
///   pair `lsh_only` (`token_jaccard >= 0.90` to survive, instead of the
///   fused gate), and `classify_signals` routes the cluster
///   `loosely_similar` (hidden) instead of `same_behavior` (visible).
///   The identical pair discovered by ANN alone survives and is shown —
///   discovery order decided whether the user saw a real duplicate.
///   A false-negative machine.
///
/// Pinned by `issue_343_sum_clamp_saturation.rs::
/// byte_identical_pair_still_earns_full_confidence_under_the_bound`,
/// which watched the structural arm render `embedding_cos = 0.0000` for
/// two byte-identical files.
///
/// # Panics
///
/// Whenever a measured cosine reaches a pair whose evidence the deleted
/// arms would have discarded. The accuracy quarantine mandates the panic:
/// silently destroying measured evidence is worse than a crash.
#[allow(
    clippy::panic,
    reason = "CLAUDE.md accuracy quarantine: measured-evidence discard causes false \
              negatives and false reported figures, and quarantined code must panic. \
              The workspace panic = \"deny\" gate would otherwise reject the mandated \
              panic; this allow is legal only on quarantined code."
)]
fn add_embedding_pair(
    pair: &EmbeddingPair,
    fingerprints: &[Fingerprint],
    scores: &mut HashMap<(usize, usize), f64>,
    cosines: &mut HashMap<(usize, usize), f64>,
) {
    let key = order(pair.left, pair.right);
    match scores.get(&key).copied() {
        Some(structural) if structural > 0.0 => panic!(
            "QUARANTINED: add_embedding_pair discarded the measured cosine {cosine} for a \
             structurally-discovered pair (structural={structural}), rendering \
             embedding_cos = 0.0 as if the similarity had been measured and found absent. \
             Pinned by issue_343_sum_clamp_saturation.rs::\
             byte_identical_pair_still_earns_full_confidence_under_the_bound.",
            cosine = pair.cosine,
        ),
        Some(_) if same_file_pair(key, fingerprints) => record_cosine(key, pair.cosine, cosines),
        Some(_) => panic!(
            "QUARANTINED: add_embedding_pair discarded the measured cosine {cosine} for a \
             cross-file pair LSH also surfaced, reclassifying it lsh_only and hiding its \
             cluster — a false negative decided by discovery order. Pinned by \
             issue_343_sum_clamp_saturation.rs::\
             byte_identical_pair_still_earns_full_confidence_under_the_bound.",
            cosine = pair.cosine,
        ),
        None => {
            let _previous = scores.insert(key, 0.0_f64);
            record_cosine(key, pair.cosine, cosines);
        }
    }
}

/// Returns true when both endpoints belong to the same source file.
fn same_file_pair(key: (usize, usize), fingerprints: &[Fingerprint]) -> bool {
    let Some(left) = fingerprints.get(key.0) else {
        return false;
    };
    let Some(right) = fingerprints.get(key.1) else {
        return false;
    };
    left.file_id == right.file_id
}

/// Keeps the highest cosine seen for a non-structural pair.
fn record_cosine(key: (usize, usize), cosine: f64, cosines: &mut HashMap<(usize, usize), f64>) {
    let _entry = cosines
        .entry(key)
        .and_modify(|current| *current = current.max(cosine))
        .or_insert(cosine);
}

/// Converts raw `(left, right) → structural_score` map into a sorted
/// [`CandidatePair`] list with token Jaccard filled in from the signatures
/// and the minimum endpoint node count attached for downstream filtering.
fn finalise_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    scores: HashMap<(usize, usize), f64>,
    cosines: &HashMap<(usize, usize), f64>,
) -> Vec<CandidatePair> {
    let mut pairs: Vec<CandidatePair> = scores
        .into_iter()
        .map(|((left, right), structural)| CandidatePair {
            left,
            right,
            min_node_count: min_node_count(fingerprints, left, right),
            lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
            fused_min_score: FUSED_THRESHOLD,
            score: PairScore {
                structural,
                token_jaccard: jaccard_for(signatures, left, right),
                embedding_cos: cosines.get(&(left, right)).copied().unwrap_or(0.0),
            },
        })
        .filter(|pair| candidate_ranges_are_valid(pair, fingerprints))
        .collect();
    pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
    pairs
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

/// Returns the smaller endpoint node count. Defaults to 0 when either
/// index is out of bounds — an impossible state in the current pipeline,
/// but keeps the helper total.
fn min_node_count(fingerprints: &[Fingerprint], left: usize, right: usize) -> usize {
    let left_count = fingerprints
        .get(left)
        .map_or(0, |fingerprint| fingerprint.node_count);
    let right_count = fingerprints
        .get(right)
        .map_or(0, |fingerprint| fingerprint.node_count);
    left_count.min(right_count)
}

/// Looks up both signatures and returns their estimated Jaccard. Returns
/// 0.0 when either signature is missing, which cannot happen in practice
/// because the pipeline always produces one signature per fingerprint.
fn jaccard_for(signatures: &[Signature], left: usize, right: usize) -> f64 {
    let Some(left_signature) = signatures.get(left) else {
        return 0.0;
    };
    let Some(right_signature) = signatures.get(right) else {
        return 0.0;
    };
    estimate_jaccard(left_signature, right_signature)
}

/// Puts the smaller index first. Pair keys are order-insensitive.
fn order(left: usize, right: usize) -> (usize, usize) {
    (left.min(right), left.max(right))
}
