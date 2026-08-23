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

/// Streams every pair of signature indexes `(i, j)`, `i < j`, whose
/// signatures collide in at least one LSH band
/// ([PERF-FLUTTER-TODO-PAIRS]).
///
/// The historical [`band_collisions`] materialised the full pair vector —
/// 55 million pairs, ~880 MB, on the Flutter corpus — before any consumer
/// touched it. This source instead walks one band at a time: the band's
/// `(hash, index)` tags are sorted in a reused buffer (a few tens of MB),
/// each equal-hash run is verified against the full 32-byte band key so a
/// truncated sort hash can never manufacture a collision, and the run's
/// star pairs are handed straight to `emit`. Memory is bounded by one
/// band's tags, not by the pair count; the caller deduplicates a pair that
/// collides in several bands.
///
/// Emission order is deterministic: bands ascending, runs in sorted order,
/// each run's pairs from its smallest member outward.
pub fn for_each_band_collision(signatures: &[Signature], emit: &mut dyn FnMut(usize, usize)) {
    let mut tagged: Vec<(u64, u32)> = Vec::with_capacity(signatures.len());
    for band in 0..BANDS {
        tagged.clear();
        for (index, signature) in signatures.iter().enumerate() {
            let key = band_key(signature, band);
            tagged.push((truncated_band_hash(&key), index_to_tag(index)));
        }
        tagged.sort_unstable();
        emit_run_pairs(signatures, band, &tagged, emit);
    }
}

/// Emits the star pairs of every equal-hash run in one band's sorted tags.
fn emit_run_pairs(
    signatures: &[Signature],
    band: usize,
    tagged: &[(u64, u32)],
    emit: &mut dyn FnMut(usize, usize),
) {
    let mut run_start = 0_usize;
    while let Some(start_tag) = tagged.get(run_start) {
        let run_hash = start_tag.0;
        let run_end = tagged
            .get(run_start.saturating_add(1)..)
            .unwrap_or(&[])
            .iter()
            .position(|&(hash, _)| hash != run_hash)
            .map_or(
                tagged.len(),
                |offset| run_start.saturating_add(1).saturating_add(offset),
            );
        emit_one_run(signatures, band, tagged, run_start, run_end, emit);
        if run_end >= tagged.len() {
            break;
        }
        run_start = run_end;
    }
}

/// Emits one equal-hash run's star pairs, verifying full band keys.
///
/// The sorted tag hash is a truncation of the 32-byte band key, so a run
/// can hold members with different full keys. Adjacent-key verification
/// splits such runs; when a split is needed the whole run is regrouped by
/// full key so two equal keys separated by a colliding third still pair —
/// exactness cannot depend on hash luck.
fn emit_one_run(
    signatures: &[Signature],
    band: usize,
    tagged: &[(u64, u32)],
    start: usize,
    end: usize,
    emit: &mut dyn FnMut(usize, usize),
) {
    if end.saturating_sub(start) < 2 {
        return;
    }
    if run_keys_split(signatures, band, tagged, start, end) {
        emit_regrouped_run(signatures, band, tagged, start, end, emit);
        return;
    }
    emit_star_pairs(tagged, start, end, emit);
}

/// True when adjacent members of the run disagree on the full band key.
fn run_keys_split(
    signatures: &[Signature],
    band: usize,
    tagged: &[(u64, u32)],
    start: usize,
    end: usize,
) -> bool {
    let slice = tagged.get(start..end).unwrap_or(&[]);
    let mut previous: Option<u32> = None;
    for tag in slice {
        if let Some(prior) = previous {
            if band_key_at(signatures, band, prior) != band_key_at(signatures, band, tag.1) {
                return true;
            }
        }
        previous = Some(tag.1);
    }
    false
}

/// Regroups a key-collided run by full band key and emits each group's
/// star pairs.
fn emit_regrouped_run(
    signatures: &[Signature],
    band: usize,
    tagged: &[(u64, u32)],
    start: usize,
    end: usize,
    emit: &mut dyn FnMut(usize, usize),
) {
    let mut groups: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for position in start..end {
        let index = tagged.get(position).map_or(0, |tag| tag.1);
        let key = band_key_at(signatures, band, index);
        groups.entry(key).or_default().push(index_as_usize(index));
    }
    let mut members: Vec<Vec<usize>> = groups.into_values().collect();
    members.sort_unstable();
    for group in members {
        emit_sorted_star(&group, emit);
    }
}

/// Emits star pairs for one tag range whose keys are known equal.
fn emit_star_pairs(
    tagged: &[(u64, u32)],
    start: usize,
    end: usize,
    emit: &mut dyn FnMut(usize, usize),
) {
    let mut members: Vec<usize> = tagged
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|tag| index_as_usize(tag.1))
        .collect();
    members.sort_unstable();
    emit_sorted_star(&members, emit);
}

/// Emits `(smallest, other)` for every member of a sorted group.
fn emit_sorted_star(members: &[usize], emit: &mut dyn FnMut(usize, usize)) {
    let Some(canonical) = members.first().copied() else {
        return;
    };
    for &other in members.iter().skip(1) {
        emit(canonical, other);
    }
}

/// The band key of signature `index`, tolerating an out-of-range index.
fn band_key_at(signatures: &[Signature], band: usize, index: u32) -> [u8; 32] {
    signatures
        .get(index_as_usize(index))
        .map_or([0_u8; 32], |signature| band_key(signature, band))
}

/// Truncates a band key to the sort hash. Collisions are handled by full
/// key verification, never by luck.
fn truncated_band_hash(key: &[u8; 32]) -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(key.get(..8).unwrap_or(&[0_u8; 8]));
    u64::from_le_bytes(bytes)
}

/// Lossless `usize → u32` tag for the sort buffer; saturates for corpora
/// past four billion signatures, which the fingerprint memory ceiling
/// excludes long before.
fn index_to_tag(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
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

    /// Low byte of `index`. Total where a fallible conversion is not,
    /// and exact over this test's domain — the signature is 32 rows of
    /// 8 bytes, so every index it feeds is below 256.
    fn byte_index(index: usize) -> u8 {
        index.to_le_bytes()[0]
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

/// Widens a signature index tag losslessly; a saturated tag indexes past
/// every real signature, which [`band_key_at`] answers as the zero key.
fn index_as_usize(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

/// [`crate::pair::LshPairs`] adapter over the streaming band source —
/// the render pass's LSH leg, without materialising the pair list.
#[derive(Debug)]
pub struct BandCollisionSource<'a> {
    /// The signatures whose band collisions are streamed.
    signatures: &'a [Signature],
}

impl<'a> BandCollisionSource<'a> {
    /// Wraps `signatures` for streaming.
    #[must_use]
    pub const fn new(signatures: &'a [Signature]) -> Self {
        Self { signatures }
    }
}

impl crate::pair::LshPairs for BandCollisionSource<'_> {
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize)) {
        for_each_band_collision(self.signatures, emit);
    }
}
