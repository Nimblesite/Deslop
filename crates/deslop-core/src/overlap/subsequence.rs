//! Ordered-subsequence upper bound on shared node mass
//! ([FUSION-SHARED-SUBTREE-BOUND-ORDER]).
//!
//! [FUSION-SHARED-SUBTREE-BOUND] bounds the alignment by the node-kind
//! multiset the two endpoints share. That bound ignores *order*, and
//! order is exactly what an ordered tree alignment must respect: in any
//! Tai mapping, one node precedes another in post-order on the left if
//! and only if its partner does on the right — a mapped pair is either
//! an ancestor pair or a left-of pair, and both orderings agree with
//! post-order. So the kind-preserving part of the mapping is a common
//! *subsequence* of the two post-order kind sequences, and the longest
//! such subsequence is a second sound upper bound — never looser than
//! the multiset one, often far tighter, because a shared multiset that
//! only occurs in scrambled order cannot be aligned.
//!
//! The point is cost. On the Flutter corpus the rescue ran 856,990
//! Zhang–Shasha alignments at roughly four milliseconds each; this
//! bound answers in microseconds because it is computed bit-parallel
//! (Allison–Dix), sixty-four post-order positions per machine word.
//! Every pair it rejects is a pair that provably could not have
//! cleared the admission floor, so the rescue's decisions are
//! unchanged and only the arithmetic reaching them is cheaper
//! ([PERF-FLUTTER-TODO-RESCUE]).

use std::collections::HashMap;

use super::alignment::PostNode;

/// Post-order positions carried by one machine word of a row vector.
const WORD_BITS: usize = 64;

/// Per-kind position masks over one endpoint's post-order sequence.
///
/// Built once per endpoint view and reused by every pair the endpoint
/// takes part in — a star-shaped candidate bucket resolves one endpoint
/// against hundreds of partners, and rebuilding the masks per pair
/// would cost more than the bound saves.
#[derive(Debug)]
pub(super) struct KindPositions {
    /// One mask per distinct kind: bit `position` is set when that
    /// post-order position carries the kind. Absent kind means "this
    /// endpoint has none", whose mask is all-zero.
    masks: HashMap<&'static str, Vec<u64>>,
    /// Post-order positions covered — the endpoint's node total,
    /// excluding the synthetic window root.
    len: usize,
    /// Words per mask: `len` rounded up to whole words.
    words: usize,
}

impl KindPositions {
    /// Indexes the first `len` post-order positions by node kind.
    pub(super) fn new(postorder: &[PostNode], len: usize) -> Self {
        let words = len.div_ceil(WORD_BITS);
        let mut masks: HashMap<&'static str, Vec<u64>> = HashMap::new();
        for (position, node) in postorder.iter().take(len).enumerate() {
            let mask = masks.entry(node.kind).or_insert_with(|| vec![0; words]);
            if let Some(word) = mask.get_mut(position / WORD_BITS) {
                *word |= 1_u64 << (position % WORD_BITS);
            }
        }
        Self { masks, len, words }
    }

    /// The positions carrying `kind`, or `None` when the endpoint has
    /// none — a kind the other side never uses cannot extend any
    /// common subsequence, so the row is left untouched.
    fn positions(&self, kind: &'static str) -> Option<&[u64]> {
        self.masks.get(kind).map(Vec::as_slice)
    }
}

/// Longest common subsequence of `left`'s first `left_len` post-order
/// kinds and the sequence `right` indexes — a sound upper bound on the
/// kind-preserving part of any ordered alignment of the two endpoints.
pub(super) fn common_subsequence_len(
    left: &[PostNode],
    left_len: usize,
    right: &KindPositions,
    row: &mut Row,
) -> usize {
    row.reset(right.words, right.len);
    for node in left.iter().take(left_len) {
        if let Some(mask) = right.positions(node.kind) {
            row.advance(mask);
        }
    }
    right.len.saturating_sub(row.unclaimed())
}

/// The Allison–Dix row state, reused across pairs so the bound
/// allocates nothing in the measurement loop.
///
/// A set bit marks a right-hand position the common subsequence has not
/// claimed, so the subsequence length is the count of cleared bits.
#[derive(Debug, Default)]
pub(super) struct Row {
    /// Row bits, one per right-hand post-order position.
    bits: Vec<u64>,
    /// Live words in `bits`.
    words: usize,
    /// Live bits — positions past this are carry spill, never counted.
    len: usize,
}

impl Row {
    /// Opens a row over `len` positions, every position unclaimed.
    fn reset(&mut self, words: usize, len: usize) {
        self.bits.clear();
        self.bits.resize(words, u64::MAX);
        self.words = words;
        self.len = len;
    }

    /// Folds one left-hand kind in: `row = (row + (row & mask)) | (row
    /// & !mask)`, as a single multi-word addition so a carry crosses
    /// word boundaries the way it would in one wide register.
    fn advance(&mut self, mask: &[u64]) {
        let mut carry = 0_u64;
        for index in 0..self.words {
            let value = word(&self.bits, index);
            let matched = word(mask, index);
            let (summed, wrapped) = value.overflowing_add(value & matched);
            let (summed, carried) = summed.overflowing_add(carry);
            carry = u64::from(wrapped || carried);
            if let Some(slot) = self.bits.get_mut(index) {
                *slot = summed | (value & !matched);
            }
        }
    }

    /// Positions still unclaimed, ignoring carry spill past `len`.
    fn unclaimed(&self) -> usize {
        (0..self.words)
            .map(|index| (word(&self.bits, index) & self.live(index)).count_ones())
            .map(|ones| usize::try_from(ones).unwrap_or(0))
            .fold(0_usize, usize::saturating_add)
    }

    /// The live-position mask for word `index`: every bit but the last
    /// word's, which stops at `len`.
    fn live(&self, index: usize) -> u64 {
        let spare = self
            .words
            .saturating_sub(index.saturating_add(1))
            .saturating_mul(WORD_BITS);
        let live_here = self.len.saturating_sub(index.saturating_mul(WORD_BITS));
        if spare > 0 || live_here >= WORD_BITS {
            return u64::MAX;
        }
        (1_u64 << live_here).saturating_sub(1)
    }
}

/// Word `index` of `words`, zero past the end.
fn word(words: &[u64], index: usize) -> u64 {
    words.get(index).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests;
