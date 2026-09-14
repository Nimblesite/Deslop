//! `MinHash` + banded LSH for Type-3 candidate discovery.
//!
//! Implements the token LSH stage of [FUSED-SIGNALS-THREE-LAYER] and the
//! second pass of [DECISION-TYPE3-TWO-PASS]. Computes a deterministic
//! `MinHash` signature over k-grams of normalised node kinds and splits the
//! signature into `BANDS × ROWS_PER_BAND` bands; pairs of fingerprints
//! sharing at least one identical band are returned as candidate Type-3
//! clones ([`banding`]). Jaccard similarity is estimated from the full
//! signatures.
//!
//! `MinHash` signature construction uses `blake3` so two different processes
//! produce identical signatures given the same input — a prerequisite for
//! caching per [PRINCIPLES-LONG-RUNNING-DAEMON]. Band keys preserve their four
//! rows directly because those rows already fill the 32-byte bucket key.

mod banding;

pub use banding::{for_each_band_collision, BandCollisionSource};

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

/// The all-zero signature — the neutral stand-in tests use for a
/// fingerprint whose signature was never built.
pub const ZEROED_SIGNATURE: Signature = [0; SIGNATURE_LEN];

/// A borrowed, positionally-indexed view over a signature population
/// stored as contiguous segments ([PERF-FLUTTER-TODO-MEMORY]).
///
/// A corpus-scale build parses files in parallel shards; requiring one
/// contiguous signature vector would force an ordered merge that
/// momentarily holds the whole multi-GB population twice. Segments —
/// each shard's signatures, concatenated in shard order — give the same
/// positional index space (`0..len`) with zero merging, and every
/// consumer reads through this view.
pub struct SignatureIndex<'a> {
    /// The segments, in index order. Owned reference list so a view can
    /// be built from a locally assembled set of segment slices.
    /// Read directly by the [`banding`] pass, which walks segments as
    /// slices.
    segments: Vec<&'a [Signature]>,
    /// Cumulative start offset per segment; `offsets[k]` is the global
    /// index where segment `k` begins.
    offsets: Vec<usize>,
}

impl std::fmt::Debug for SignatureIndex<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SignatureIndex")
            .field("signatures", &self.len())
            .finish()
    }
}

impl<'a> SignatureIndex<'a> {
    /// Builds the view over `segments`, precomputing the offsets.
    #[must_use]
    pub fn from_segments(segments: impl IntoIterator<Item = &'a [Signature]>) -> Self {
        let segments: Vec<&'a [Signature]> = segments.into_iter().collect();
        let mut offsets = Vec::with_capacity(segments.len().saturating_add(1));
        let mut running = 0_usize;
        offsets.push(0);
        for segment in &segments {
            running = running.saturating_add(segment.len());
            offsets.push(running);
        }
        Self { segments, offsets }
    }

    /// Builds the view over a single contiguous slice — the natural
    /// shape for tests and small corpora with no sharding.
    #[must_use]
    pub fn from_slice(slice: &'a [Signature]) -> Self {
        Self::from_segments([slice])
    }

    /// Total signatures across every segment.
    #[must_use]
    pub fn len(&self) -> usize {
        self.offsets.last().copied().unwrap_or(0)
    }

    /// Whether the population is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The signature at global `index`, or `None` past the end.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Signature> {
        let segment = self
            .offsets
            .partition_point(|&start| start <= index)
            .saturating_sub(1);
        let start = self.offsets.get(segment).copied().unwrap_or(0);
        self.segments
            .get(segment)
            .and_then(|slice| slice.get(index.saturating_sub(start)))
    }
}

/// Random-access signature reads over a population, whatever backs it
/// ([PERF-FLUTTER-TODO-MEMORY]). Today every implementor is a resident
/// segment view; the indirection is the seam a future non-resident
/// backing would implement — with the caveat that the banding and
/// pair-gate consumers read the population ~10⁸ times on a
/// corpus-scale run, so any such backing must serve reads at memory
/// speed and lend a reference rather than copy. A signature is a
/// kilobyte, and the cluster-signal stage alone asked for 32 million of
/// them on the Flutter corpus ([PERF-FLUTTER-TODO-PAIRS]); handing back
/// a borrow rather than filling a caller buffer removes 32 GB of
/// memcpy from that stage without changing a single measured value.
pub trait SignatureLookup: Sync {
    /// The type's name, for `Debug`.
    fn kind(&self) -> &'static str {
        "SignatureLookup"
    }

    /// Total signatures in the population.
    fn len(&self) -> usize;

    /// Whether the population is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The signature at `index`, or `None` when the population has no
    /// such position.
    fn signature(&self, index: usize) -> Option<&Signature>;
}

impl std::fmt::Debug for dyn SignatureLookup + '_ {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.kind())
    }
}

impl SignatureLookup for SignatureIndex<'_> {
    fn kind(&self) -> &'static str {
        "SignatureIndex"
    }

    fn len(&self) -> usize {
        SignatureIndex::len(self)
    }

    fn signature(&self, index: usize) -> Option<&Signature> {
        self.get(index)
    }
}

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
        let mut hasher = blake3::Hasher::new();
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
