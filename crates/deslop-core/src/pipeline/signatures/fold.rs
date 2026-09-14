//! The composable token-stream state behind the bottom-up signature
//! fold ([PIPELINE-SIGNATURE-FOLD], [PERF-FLUTTER-TODO-CORPUS]): the
//! `MinHash` of a sequence is the element-wise minimum over its
//! k-grams, so a parent's state is recomputable from its children's
//! states plus the few k-grams straddling their boundaries. Split from
//! the parent module, which owns the tree walk and member emission.

use crate::{
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    tokens::KGRAM_WIDTH,
};

/// Boundary tokens one [`TokenState`] must retain: exactly `KGRAM_WIDTH - 1`
/// from each end of its token sequence, which is the most any
/// junction-spanning k-gram can read.
pub(super) const EDGE_LEN: usize = KGRAM_WIDTH - 1;

/// The first/last [`EDGE_LEN`] tokens of a token sequence, stored inline
/// so [`TokenState`] stays copyable and allocation-free. Unused slots hold
/// `""`, which no normalised kind ever is.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TokenEnds {
    /// The edge tokens, padded with `""`.
    tokens: [&'static str; EDGE_LEN],
    /// How many slots are real tokens.
    len: usize,
}

impl TokenEnds {
    /// The `count` leading tokens of `self` followed by `next`'s, truncated
    /// to [`EDGE_LEN`].
    fn prefix_joined(mut self, next: Self) -> Self {
        for token in next.tokens.into_iter().take(next.len) {
            match self.tokens.get_mut(self.len) {
                Some(slot) => {
                    *slot = token;
                    self.len = self.len.saturating_add(1);
                }
                None => break,
            }
        }
        self
    }

    /// The last [`EDGE_LEN`] tokens of `self ++ next`:
    /// `next`'s whole tail when it already fills the window, else
    /// `self`'s trailing tokens followed by all of `next`'s — in sequence
    /// order, unlike the prefix which truncates from the right.
    fn suffix_joined(self, next: Self) -> Self {
        let mut tokens = [""; EDGE_LEN];
        let mut len = 0_usize;
        let needed = EDGE_LEN.saturating_sub(next.len);
        let from = self.len.saturating_sub(needed);
        for token in self.tokens.into_iter().take(self.len).skip(from) {
            match tokens.get_mut(len) {
                Some(slot) => {
                    *slot = token;
                    len = len.saturating_add(1);
                }
                None => break,
            }
        }
        for token in next.tokens.into_iter().take(next.len) {
            match tokens.get_mut(len) {
                Some(slot) => {
                    *slot = token;
                    len = len.saturating_add(1);
                }
                None => break,
            }
        }
        TokenEnds { tokens, len }
    }
}

/// One subtree's composable token-stream measurement state: the `MinHash`
/// over its interior k-grams, the boundary tokens junction k-grams need,
/// and its token count. Joining two adjacent states reproduces the
/// top-down `MinHash` of the concatenated sequence exactly, because the
/// concatenated k-grams are precisely the left k-grams, the right k-grams,
/// and the junction-straddling ones.
#[derive(Debug, Clone, Copy)]
pub(super) struct TokenState {
    /// Element-wise minimum over the sequence's k-gram signatures;
    /// all-`u64::MAX` when the sequence holds no k-gram.
    pub(super) signature: Signature,
    /// First `k-1` tokens of the sequence.
    prefix: TokenEnds,
    /// Last `k-1` tokens of the sequence.
    suffix: TokenEnds,
    /// Total tokens in the sequence.
    pub(super) count: usize,
}

impl TokenState {
    /// The state of a sequence holding no tokens.
    pub(super) fn empty() -> Self {
        Self {
            signature: [u64::MAX; SIGNATURE_LEN],
            prefix: TokenEnds::default(),
            suffix: TokenEnds::default(),
            count: 0,
        }
    }

    /// The state of a sequence holding exactly `kind`.
    pub(super) fn singleton(kind: &'static str) -> Self {
        let ends = TokenEnds {
            tokens: [kind; EDGE_LEN],
            len: 1,
        };
        Self {
            signature: [u64::MAX; SIGNATURE_LEN],
            prefix: ends,
            suffix: ends,
            count: 1,
        }
    }
}

/// Element-wise minimum of `other` into `target`.
fn min_into(target: &mut Signature, other: &Signature) {
    for (slot, value) in target.iter_mut().zip(other.iter()) {
        if *value < *slot {
            *slot = *value;
        }
    }
}

/// Concatenation of two adjacent token sequences as a [`TokenState`].
pub(super) fn join_states(left: &TokenState, right: &TokenState) -> TokenState {
    if left.count == 0 {
        return *right;
    }
    if right.count == 0 {
        return *left;
    }
    let count = left.count.saturating_add(right.count);
    let mut signature = left.signature;
    min_into(&mut signature, &right.signature);
    if count >= KGRAM_WIDTH {
        fold_junction_grams(left, right, count, &mut signature);
    }
    TokenState {
        signature,
        prefix: left.prefix.prefix_joined(right.prefix),
        suffix: left.suffix.suffix_joined(right.suffix),
        count,
    }
}

/// Folds the k-grams straddling the left/right junction into `signature`.
/// Those grams — and only those — start at combined positions
/// `[max(0, left.count - k + 1), min(left.count - 1, count - k)]`; every
/// one reads solely left's last `k-1` tokens and right's first `k-1`.
fn fold_junction_grams(
    left: &TokenState,
    right: &TokenState,
    count: usize,
    signature: &mut Signature,
) {
    let tail = &left.suffix;
    let head = &right.prefix;
    let junction_len = tail.len.saturating_add(head.len);
    if junction_len < KGRAM_WIDTH {
        return;
    }
    let first = left.count.saturating_sub(EDGE_LEN);
    let last = (left.count.saturating_sub(1)).min(count.saturating_sub(KGRAM_WIDTH));
    let tail_offset = left.count.saturating_sub(tail.len);
    for start in first..=last {
        let junction_start = start.saturating_sub(tail_offset);
        if junction_start.saturating_add(KGRAM_WIDTH) > junction_len {
            continue;
        }
        let gram = junction_gram(*tail, *head, junction_start);
        let slice: &[&'static str] = gram.as_slice();
        min_into(signature, &minhash_signature(std::slice::from_ref(&slice)));
    }
}

/// The k-gram starting at `junction_start` within `tail ++ head`.
fn junction_gram(
    tail: TokenEnds,
    head: TokenEnds,
    junction_start: usize,
) -> [&'static str; KGRAM_WIDTH] {
    let mut gram = [""; KGRAM_WIDTH];
    let mut filled = 0_usize;
    for token in tail.tokens.into_iter().take(tail.len).skip(junction_start) {
        if let Some(slot) = gram.get_mut(filled) {
            *slot = token;
            filled = filled.saturating_add(1);
        }
    }
    for token in head.tokens.into_iter().take(head.len) {
        if filled >= KGRAM_WIDTH {
            break;
        }
        if let Some(slot) = gram.get_mut(filled) {
            *slot = token;
            filled = filled.saturating_add(1);
        }
    }
    gram
}
