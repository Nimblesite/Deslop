//! [FUSED-CONTENT-GATE-LITERAL-BOUND] Per-scan memo of each endpoint's heaviest literal.

use std::hash::BuildHasher;

use super::GateShared;
use crate::{
    content::literal_bound::{carried_text_differs, heaviest_literal, HeaviestLiteral},
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
}

impl LiteralBoundMemo {
    /// Reserves one slot per fingerprint in the candidate corpus.
    pub(super) fn new(count: usize) -> Self {
        Self {
            heaviest: vec![Slot::Unresolved; count],
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
        carried_text_differs(left, right, shared.sources)
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
