//! MinHash + banded LSH for Type-3 candidate discovery.
//!
//! Implements the token LSH stage of [FUSION-SIGNALS-THREE-LAYER] and the
//! second pass of [DECISION-TYPE3-TWO-PASS]. Computes a deterministic
//! MinHash signature over k-grams of normalised node kinds and splits the
//! signature into `BANDS × ROWS_PER_BAND` bands; pairs of fingerprints
//! sharing at least one identical band are returned as candidate Type-3
//! clones. Jaccard similarity is estimated from the full signatures.
//!
//! All hashing uses `blake3` so two different processes produce identical
//! signatures given the same input — a prerequisite for caching per
//! [PRINCIPLES-LONG-RUNNING-DAEMON].

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

/// MinHash signature for one subtree. Fixed-length array to avoid per-call
/// allocation in the hot loop.
pub type Signature = [u64; SIGNATURE_LEN];

/// Computes a [`Signature`] for a set of k-grams of normalised node kinds.
/// Deterministic: given the same input it always returns the same output,
/// across processes and architectures.
#[must_use]
pub fn minhash_signature(kgrams: &[&[&'static str]]) -> Signature {
    let mut signature: Signature = [EMPTY_HASH_SENTINEL; SIGNATURE_LEN];
    for gram in kgrams {
        let kgram_bytes = kgram_bytes(gram);
        for (index, slot) in signature.iter_mut().enumerate() {
            let candidate = seeded_hash(&kgram_bytes, index);
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

/// Extracts and deduplicates all pairs from LSH buckets. Split out of
/// [`band_collisions`] so each function stays under the 20-line budget.
fn collect_pairs(buckets: &HashMap<[u8; 32], Vec<usize>>) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for members in buckets.values() {
        if members.len() < 2 {
            continue;
        }
        for (a_pos, a_index) in members.iter().enumerate() {
            for b_index in members.iter().skip(a_pos.saturating_add(1)) {
                pairs.push(ordered_pair(*a_index, *b_index));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Normalises a pair so the smaller index is first. Keeps the downstream
/// candidate set symmetric without extra bookkeeping.
const fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Hashes one band of a signature into a stable 32-byte key used as a
/// HashMap bucket.
fn band_key(signature: &Signature, band: usize) -> [u8; 32] {
    let mut hasher = Hasher::new();
    let start = band.saturating_mul(ROWS_PER_BAND);
    for offset in 0..ROWS_PER_BAND {
        let slot_index = start.saturating_add(offset);
        let value = signature.get(slot_index).copied().unwrap_or(0);
        let _ = hasher.update(&value.to_le_bytes());
    }
    hasher.finalize().into()
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

/// Returns a deterministic 64-bit hash of `payload` seeded with `index`.
/// Uses `blake3` throughout for one dependency surface.
fn seeded_hash(payload: &[u8], index: usize) -> u64 {
    let mut hasher = Hasher::new();
    let seed = u64::try_from(index).unwrap_or(u64::MAX);
    let _ = hasher.update(&seed.to_le_bytes());
    let _ = hasher.update(payload);
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();
    let mut narrow = [0_u8; 8];
    let slice = bytes.get(..8).unwrap_or(&[0_u8; 8]);
    narrow.copy_from_slice(slice);
    u64::from_le_bytes(narrow)
}
