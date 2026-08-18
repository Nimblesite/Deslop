//! `MinHash` + banded LSH for Type-3 candidate discovery.
//!
//! Implements the token LSH stage of [FUSION-SIGNALS-THREE-LAYER] and the
//! second pass of [DECISION-TYPE3-TWO-PASS]. Computes a deterministic
//! `MinHash` signature over k-grams of normalised node kinds and splits the
//! signature into `BANDS × ROWS_PER_BAND` bands; pairs of fingerprints
//! sharing at least one identical band are returned as candidate Type-3
//! clones. Jaccard similarity is estimated from the full signatures.
//!
//! `MinHash` signature construction uses `blake3` so two different processes
//! produce identical signatures given the same input — a prerequisite for
//! caching per [PRINCIPLES-LONG-RUNNING-DAEMON]. Band keys preserve their four
//! rows directly because those rows already fill the 32-byte bucket key.

use std::collections::HashMap;

use blake3::Hasher;

/// Signature length. Product of [`BANDS`] and [`ROWS_PER_BAND`]; a 128-length
/// signature gives a band-collision probability curve that starts rising
/// sharply near Jaccard = 0.5 and saturates by 0.85 — a reasonable operating
/// point for Type-3 recall without flooding the candidate set with false
/// positives.
pub const SIGNATURE_LEN: usize = 128;
/// Number of LSH bands. Higher values recall more but produce more
/// candidates per bucket.
pub const BANDS: usize = 32;
/// Rows per band. `BANDS * ROWS_PER_BAND == SIGNATURE_LEN` must hold.
pub const ROWS_PER_BAND: usize = 4;

/// Sentinel used when a k-gram set is empty (i.e. the subtree is smaller
/// than `k` tokens). Saturates at `u64::MAX` so downstream min-comparisons
/// treat the feature as "infinitely far."
const EMPTY_HASH_SENTINEL: u64 = u64::MAX;

/// `MinHash` signature for one subtree. Fixed-length array to avoid
/// per-call allocation in the hot loop.
pub type Signature = [u64; SIGNATURE_LEN];

/// Computes a [`Signature`] for a set of k-grams of normalised node kinds.
/// Deterministic: given the same input it always returns the same output,
/// across processes and architectures.
///
/// Uses blake3 XOF to derive all 128 slot values from a single hash call
/// per k-gram — 128× fewer hasher allocations than the naïve seeded approach.
#[must_use]
pub fn minhash_signature(kgrams: &[&[&'static str]]) -> Signature {
    let mut signature: Signature = [EMPTY_HASH_SENTINEL; SIGNATURE_LEN];
    let mut expanded = [0u8; SIGNATURE_LEN * 8];
    for gram in kgrams {
        let gram_bytes = kgram_bytes(gram);
        let mut hasher = Hasher::new();
        let _ = hasher.update(&gram_bytes);
        hasher.finalize_xof().fill(&mut expanded);
        for (slot, chunk) in signature.iter_mut().zip(expanded.chunks_exact(8)) {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(chunk);
            let candidate = u64::from_le_bytes(arr);
            if candidate < *slot {
                *slot = candidate;
            }
        }
    }
    signature
}

/// Returns Jaccard similarity estimate in `[0.0, 1.0]` between two
/// signatures. Exact equality counts; sentinel-vs-sentinel slots count as
/// agreement because they both mean "the set was empty" and agreement is
/// still the correct answer (both sets contained no features).
#[must_use]
pub fn estimate_jaccard(left: &Signature, right: &Signature) -> f64 {
    let mut agreements: u32 = 0;
    for (l, r) in left.iter().zip(right.iter()) {
        if l == r {
            agreements = agreements.saturating_add(1);
        }
    }
    f64::from(agreements) / f64::from(u32::try_from(SIGNATURE_LEN).unwrap_or(u32::MAX))
}

/// Returns pair indices `(i, j)` with `i < j` whose signatures collide in at
/// least one band. Deterministic output order: sorted ascending by `(i, j)`.
#[must_use]
pub fn band_collisions(signatures: &[Signature]) -> Vec<(usize, usize)> {
    let mut buckets: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for (index, signature) in signatures.iter().enumerate() {
        for band in 0..BANDS {
            let key = band_key(signature, band);
            buckets.entry(key).or_default().push(index);
        }
    }
    collect_pairs(&buckets)
}

/// Extracts deduplicated pairs from LSH buckets using a star topology per
/// bucket (the canonical member is paired with every other), matching the
/// structural-pass strategy in [`crate::pair::collect_structural_pairs`].
/// This keeps the LSH pair count linear in bucket size, which matters for
/// popular bands on large corpora where a single bucket can hold
/// thousands of fingerprints.
fn collect_pairs(buckets: &HashMap<[u8; 32], Vec<usize>>) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for members in buckets.values() {
        if members.len() < 2 {
            continue;
        }
        let mut sorted = members.clone();
        sorted.sort_unstable();
        let Some(canonical) = sorted.first().copied() else {
            continue;
        };
        for other in sorted.iter().skip(1) {
            pairs.push(ordered_pair(canonical, *other));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Normalises a pair so the smaller index is first. Keeps the downstream
/// candidate set symmetric without extra bookkeeping. `usize::min`
/// and `usize::max` are branch-free CPU intrinsics, so this reads
/// cleanly in the coverage report.
fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// Encodes one band as its four little-endian rows for use as a `HashMap` key.
/// The rows fill the 32-byte key, so key equality exactly preserves band
/// equality without an additional hash.
fn band_key(signature: &Signature, band: usize) -> [u8; 32] {
    let mut key = [0; ROWS_PER_BAND * size_of::<u64>()];
    let start = band.saturating_mul(ROWS_PER_BAND);
    for (offset, key_row) in key.chunks_exact_mut(size_of::<u64>()).enumerate() {
        let slot_index = start.saturating_add(offset);
        let value = signature.get(slot_index).copied().unwrap_or(0);
        key_row.copy_from_slice(&value.to_le_bytes());
    }
    key
}

/// Flattens a k-gram into a byte buffer with a nul separator so
/// `["a","bc"]` and `["ab","c"]` hash differently.
fn kgram_bytes(gram: &[&'static str]) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    for token in gram {
        buffer.extend_from_slice(token.as_bytes());
        buffer.push(0);
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::{band_key, Signature, ROWS_PER_BAND, SIGNATURE_LEN};

    fn byte_index(index: usize) -> u8 {
        u8::try_from(index).expect("test byte index fits in u8")
    }

    #[test]
    fn band_key_is_identity_concatenation() {
        let rows: [u64; ROWS_PER_BAND] = std::array::from_fn(|row| {
            let start = byte_index(row * size_of::<u64>());
            u64::from_le_bytes(std::array::from_fn(|byte| start + byte_index(byte)))
        });
        let mut signature: Signature = [0; SIGNATURE_LEN];
        signature[ROWS_PER_BAND..ROWS_PER_BAND * 2].copy_from_slice(&rows);

        assert_eq!(band_key(&signature, 1), std::array::from_fn(byte_index));
    }
}
