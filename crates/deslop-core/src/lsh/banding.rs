//! Banded LSH collision streaming over a signature population
//! ([PERF-FLUTTER-TODO-PAIRS]) — the band-tag sort, full-key
//! verification, and star emission behind
//! [`for_each_band_collision`]. Split from the parent module so the
//! signature/`MinHash` core and the collision machinery each stay
//! within the file budget; every public name is re-exported from
//! [`crate::lsh`], which remains the API surface.

use std::collections::HashMap;

use super::{Signature, SignatureIndex, BANDS, ROWS_PER_BAND};

/// Streams every pair of signature indexes `(i, j)`, `i < j`, whose
/// signatures collide in at least one LSH band
/// ([PERF-FLUTTER-TODO-PAIRS]).
///
/// The historical `band_collisions` materialised the full pair vector —
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
pub fn for_each_band_collision(
    signatures: &SignatureIndex<'_>,
    emit: &mut dyn FnMut(usize, usize),
) {
    let mut tagged: Vec<(u64, u32)> = Vec::with_capacity(signatures.len());
    for band in 0..BANDS {
        tagged.clear();
        fill_band_tags(signatures, band, &mut tagged);
        tagged.sort_unstable();
        emit_run_pairs(signatures, band, &tagged, emit);
    }
}

/// Fills `tagged` with `(band hash, index tag)` for every signature in
/// index order — the per-band pass of [`for_each_band_collision`],
/// walking each segment as a slice.
fn fill_band_tags(signatures: &SignatureIndex<'_>, band: usize, tagged: &mut Vec<(u64, u32)>) {
    for (segment, start) in signatures.segments.iter().zip(&signatures.offsets) {
        for (within, signature) in segment.iter().enumerate() {
            let key = band_key(signature, band);
            let index = start.saturating_add(within);
            tagged.push((truncated_band_hash(&key), index_to_tag(index)));
        }
    }
}

/// Emits the star pairs of every equal-hash run in one band's sorted tags.
fn emit_run_pairs(
    signatures: &SignatureIndex<'_>,
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
            .map_or(tagged.len(), |offset| {
                run_start.saturating_add(1).saturating_add(offset)
            });
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
    signatures: &SignatureIndex<'_>,
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
    signatures: &SignatureIndex<'_>,
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
    signatures: &SignatureIndex<'_>,
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
fn band_key_at(signatures: &SignatureIndex<'_>, band: usize, index: u32) -> [u8; 32] {
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

/// Widens a signature index tag losslessly; a saturated tag indexes past
/// every real signature, which [`band_key_at`] answers as the zero key.
fn index_as_usize(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
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

/// [`crate::pair::LshPairs`] adapter over the streaming band source —
/// the render pass's LSH leg, without materialising the pair list.
pub struct BandCollisionSource<'a> {
    /// The signatures whose band collisions are streamed.
    signatures: &'a SignatureIndex<'a>,
}

impl std::fmt::Debug for BandCollisionSource<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BandCollisionSource")
    }
}

impl<'a> BandCollisionSource<'a> {
    /// Wraps `signatures` for streaming.
    #[must_use]
    pub const fn new(signatures: &'a SignatureIndex<'a>) -> Self {
        Self { signatures }
    }
}

impl crate::pair::LshPairs for BandCollisionSource<'_> {
    fn for_each(&self, emit: &mut dyn FnMut(usize, usize)) {
        for_each_band_collision(self.signatures, emit);
    }
}

#[cfg(test)]
mod streaming_tests {
    //! [PERF-FLUTTER-TODO-PAIRS] Pins for the streaming band-collision
    //! source: equal-band signatures pair through the star emission,
    //! truncated-hash collisions never manufacture a pair (full-key
    //! verification + regroup), and a pair colliding in many bands is
    //! emitted once per band for the caller to deduplicate — the
    //! documented contract
    //! (`docs/performance-branch-review.md`, "streamed LSH construction").

    use super::super::{Signature, SignatureIndex, BANDS, SIGNATURE_LEN};
    use super::for_each_band_collision;

    /// A signature whose band `band` is filled with `filler` and every
    /// other band with a value unique to `seed`.
    fn seeded(seed: u64, band: usize, filler: u64) -> Signature {
        let mut signature: Signature = [0; SIGNATURE_LEN];
        for (slot, value) in signature.iter_mut().enumerate() {
            *value = if slot / 4 == band {
                filler
            } else {
                seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add(slot as u64)
            };
        }
        signature
    }

    /// Signatures sharing one band pair exactly once — through that
    /// band's star, from the smallest member outward.
    #[test]
    fn identical_band_keys_pair_through_the_star() {
        let signatures = [seeded(1, 3, 777), seeded(2, 3, 777), seeded(3, 7, 999)];
        let mut pairs = Vec::new();
        let index = SignatureIndex::from_slice(&signatures);
        for_each_band_collision(&index, &mut |left, right| pairs.push((left, right)));
        assert_eq!(
            pairs,
            vec![(0, 1)],
            "only signatures 0 and 1 share a band; emission must be the star              from the smallest member"
        );
    }

    /// A three-member run pairs from its canonical member to each other,
    /// never member-to-member beyond the star.
    #[test]
    fn a_run_emits_star_pairs_only() {
        let signatures = [
            seeded(10, 5, 42),
            seeded(11, 5, 42),
            seeded(12, 5, 42),
            seeded(13, 0, 7),
        ];
        let mut pairs = Vec::new();
        let index = SignatureIndex::from_slice(&signatures);
        for_each_band_collision(&index, &mut |left, right| pairs.push((left, right)));
        assert_eq!(
            pairs,
            vec![(0, 1), (0, 2)],
            "the run's star is (0,1),(0,2) — (1,2) is implied, not emitted"
        );
    }

    /// A truncated sort hash colliding across different full keys must
    /// not pair the different-key signatures; exact equal keys
    /// separated by the collider still pair via the regroup. The
    /// collision is constructed, not hunted: the emitter's run logic
    /// keys off the truncated sort hash in `tagged`, so a synthetic run
    /// with equal sort hashes over signatures with different full band
    /// keys exercises the split-and-regroup path deterministically — a
    /// brute-force search of 64-bit hashes would pass without ever
    /// seeing a collision.
    #[test]
    fn truncated_hash_collisions_never_manufacture_pairs() {
        // Two clones (identical full band-0 keys) separated by one
        // unrelated signature whose full key differs, all merged into
        // one run by an equal truncated sort hash.
        const COLLIDING_SORT_HASH: u64 = 42;
        let clone_key = seeded(1, 0, 7);
        let unrelated = seeded(2, 3, 9);
        let signatures = [clone_key, unrelated, clone_key];
        let index = SignatureIndex::from_slice(&signatures);
        let tagged = [
            (COLLIDING_SORT_HASH, super::index_to_tag(0)),
            (COLLIDING_SORT_HASH, super::index_to_tag(1)),
            (COLLIDING_SORT_HASH, super::index_to_tag(2)),
        ];
        let mut pairs = Vec::new();
        super::emit_one_run(&index, 0, &tagged, 0, tagged.len(), &mut |left, right| {
            pairs.push((left, right));
        });
        assert_eq!(
            pairs,
            vec![(0, 2)],
            "the collider-separated clones (0, 2) must pair through the regroup; \
            the unrelated signature (1) must never pair across the collision"
        );
    }

    /// One pair colliding in every band is emitted once per band —
    /// `BANDS` times — and the caller's dedup collapses them. This pins
    /// the emission cardinality the dedup contract depends on.
    #[test]
    fn a_pair_colliding_in_every_band_emits_once_per_band() {
        let signatures = [seeded(5, 0, 13), seeded(5, 0, 13)];
        let mut count = 0_u64;
        let index = SignatureIndex::from_slice(&signatures);
        for_each_band_collision(&index, &mut |_left, _right| count = count.saturating_add(1));
        assert_eq!(
            count, BANDS as u64,
            "identical signatures collide in every band"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Signature, ROWS_PER_BAND, SIGNATURE_LEN};
    use super::band_key;

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
