//! [FUSED-CONTENT-GATE-LITERAL-BOUND] Per-scan memo of each endpoint's heaviest literal
//! and of its text's shingles: an endpoint in a star is read once, not once per partner.

use std::hash::BuildHasher;

use super::GateShared;
use crate::{
    content::literal_bound::{
        carried_texts, heaviest_literal, shingles, shingles_clear_floor, HeaviestLiteral,
    },
    fingerprint::Fingerprint,
    pair::CandidatePair,
};

/// One endpoint's memo slot.
#[derive(Clone, Copy)]
enum Slot {
    /// Not yet looked up.
    Unresolved,
    /// Looked up: the heaviest literal, when the endpoint holds one.
    Resolved(Option<HeaviestLiteral>),
}

/// One scan's heaviest-literal facts, computed once per fingerprint.
pub(super) struct LiteralBoundMemo {
    /// The endpoint's heaviest literal, once resolved.
    heaviest: Vec<Slot>,
    /// The carried endpoint's text shingles, once a pair needed them.
    shingles: Vec<Option<Vec<u64>>>,
}

impl LiteralBoundMemo {
    /// Reserves one slot per fingerprint in the candidate corpus.
    pub(super) fn new(count: usize) -> Self {
        Self {
            heaviest: vec![Slot::Unresolved; count],
            shingles: vec![None; count],
        }
    }

    /// True when a literal-carried endpoint is paired with different text.
    pub(super) fn refuses<L: BuildHasher>(
        &mut self,
        pair: &CandidatePair,
        left: &Fingerprint,
        right: &Fingerprint,
        shared: &GateShared<'_, L>,
    ) -> bool {
        let left = self.resolve(pair.left, left, shared);
        let right = self.resolve(pair.right, right, shared);
        let Some((left, right)) = carried_texts(left, right, shared.sources) else {
            return false;
        };
        left != right && !self.near_copies((pair.left, left), (pair.right, right))
    }

    /// Whether two differing endpoint texts share enough shingles.
    fn near_copies(&mut self, left: (usize, &[u8]), right: (usize, &[u8])) -> bool {
        self.remember(left);
        self.remember(right);
        self.known(left.0)
            .zip(self.known(right.0))
            .is_some_and(|(left, right)| shingles_clear_floor(left, right))
    }

    /// Shingles one endpoint's text the first time a pair needs it.
    fn remember(&mut self, (index, text): (usize, &[u8])) {
        if let Some(slot @ None) = self.shingles.get_mut(index) {
            *slot = Some(shingles(text));
        }
    }

    /// One endpoint's remembered shingles.
    fn known(&self, index: usize) -> Option<&[u64]> {
        self.shingles.get(index)?.as_deref()
    }

    /// Memoizes one endpoint's heaviest literal by fingerprint index.
    fn resolve<L: BuildHasher>(
        &mut self,
        index: usize,
        fingerprint: &Fingerprint,
        shared: &GateShared<'_, L>,
    ) -> Option<HeaviestLiteral> {
        if let Some(Slot::Resolved(known)) = self.heaviest.get(index) {
            return *known;
        }
        let found = heaviest_literal(&shared.tree_index, fingerprint);
        if let Some(slot) = self.heaviest.get_mut(index) {
            *slot = Slot::Resolved(found);
        }
        found
    }
}
