//! [VSIX-PAIR-COMPARE] Discovery's exact-copy anchors, kept for explicit
//! comparisons ([FUSED-SHARED-SUBTREE]).

use std::sync::OnceLock;

use crate::{fingerprint::Fingerprint, pair::CandidatePair};

/// The Merkle-equal candidate pairs of one corpus generation. An explicit
/// comparison reads them instead of running discovery again; their indices
/// address the store's fingerprints, so every splice forgets them.
#[derive(Debug, Default)]
pub(super) struct ComparisonAnchors(OnceLock<Vec<CandidatePair>>);

impl ComparisonAnchors {
    /// Keeps the exact pairs of this generation's first discovery.
    pub(super) fn publish(&self, pairs: &[CandidatePair], fingerprints: &[Fingerprint]) {
        let _already_published = self.0.set(exact(pairs.iter().copied(), fingerprints));
    }

    /// This generation's anchors, discovering them only when no render
    /// has published them yet.
    pub(super) fn get_or_discover(
        &self,
        fingerprints: &[Fingerprint],
        discover: impl FnOnce() -> Vec<CandidatePair>,
    ) -> &[CandidatePair] {
        self.0.get_or_init(|| exact(discover(), fingerprints))
    }

    /// Drops anchors whose fingerprint indices or language policy changed.
    pub(super) fn forget(&mut self) {
        self.0 = OnceLock::new();
    }
}

/// The pairs whose two endpoints are the same Merkle tree.
fn exact(
    pairs: impl IntoIterator<Item = CandidatePair>,
    fingerprints: &[Fingerprint],
) -> Vec<CandidatePair> {
    pairs
        .into_iter()
        .filter(|pair| {
            fingerprints
                .get(pair.left)
                .zip(fingerprints.get(pair.right))
                .is_some_and(|(left, right)| left.hash == right.hash)
        })
        .collect()
}
