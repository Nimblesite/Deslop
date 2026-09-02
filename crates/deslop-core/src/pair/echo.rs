//! [FUSED-SHARED-SUBTREE-ECHO] Exact whole-function clones a rescue
//! pair may not merely wrap.
//!
//! A shared-subtree rescue exists to admit a near-miss the anchor axis
//! cannot see. It is not a second way to publish a clone the anchor
//! axis already proved: a class shell or module preamble that encloses
//! a Merkle-equal authored function in both files measures high overlap
//! *because of that function*, and admitting the container hands
//! subsumption a wider, byte-divergent view that then eats the exact
//! one ([PIPELINE-CLUSTER-SUBSUME] prefers enclosure). The index below
//! records every candidate pair that is Merkle-equal, cross-file, and an
//! authored function on both sides, so the rescue can ask how much of a
//! container's shared mass is already claimed.

use std::collections::HashMap;

use crate::{
    ast::ByteRange, cluster::scope::DeclarationScopes, fingerprint::Fingerprint, state::FileId,
};

use super::CandidatePair;

/// One exact whole-function clone: the two ranges it occupies, keyed by
/// the ordered file pair, and the nodes it claims.
#[derive(Clone, Copy)]
struct ExactFunctionPair {
    /// Range in the lower-numbered file.
    first: ByteRange,
    /// Range in the higher-numbered file.
    second: ByteRange,
    /// Nodes of the clone — both endpoints agree, being Merkle-equal.
    nodes: usize,
}

/// Every exact whole-function clone among the candidate pairs, indexed
/// by ordered file pair.
pub(crate) struct ExactFunctionAnchors {
    /// Exact whole-function pairs by ordered file pair.
    by_files: HashMap<(FileId, FileId), Vec<ExactFunctionPair>>,
}

impl ExactFunctionAnchors {
    /// Indexes the Merkle-equal, cross-file, function-aligned pairs of
    /// `pairs`.
    pub(crate) fn index<L: std::hash::BuildHasher>(
        pairs: &[CandidatePair],
        fingerprints: &[Fingerprint],
        scopes: &DeclarationScopes<'_, L>,
    ) -> Self {
        let mut by_files: HashMap<(FileId, FileId), Vec<ExactFunctionPair>> = HashMap::new();
        for pair in pairs {
            let (Some(left), Some(right)) =
                (fingerprints.get(pair.left), fingerprints.get(pair.right))
            else {
                continue;
            };
            if left.file_id == right.file_id
                || left.hash != right.hash
                || scopes.aligned_function(left).is_none()
                || scopes.aligned_function(right).is_none()
            {
                continue;
            }
            let (key, first, second) = ordered(left, right);
            by_files.entry(key).or_default().push(ExactFunctionPair {
                first,
                second,
                nodes: left.node_count,
            });
        }
        Self { by_files }
    }

    /// Whether an unanchored token-only pair merely wraps an exact
    /// whole-function clone: both endpoints enclose one, and the larger
    /// endpoint holds fewer than `floor` nodes beyond it. With no
    /// measured overlap on this route, the endpoint's own node count
    /// bounds what the pair could share beyond the clone.
    pub(crate) fn wraps_within(
        &self,
        left: &Fingerprint,
        right: &Fingerprint,
        floor: usize,
    ) -> bool {
        self.claimed_nodes(left, right).is_some_and(|claimed| {
            left.node_count
                .max(right.node_count)
                .saturating_sub(claimed)
                < floor
        })
    }

    /// The most nodes any exact whole-function clone enclosed by both
    /// `left` and `right` claims, or `None` when the pair wraps none.
    pub(crate) fn claimed_nodes(&self, left: &Fingerprint, right: &Fingerprint) -> Option<usize> {
        let (key, first, second) = ordered(left, right);
        self.by_files
            .get(&key)?
            .iter()
            .filter(|exact| first.covers(exact.first) && second.covers(exact.second))
            .map(|exact| exact.nodes)
            .max()
    }
}

/// The pair's file key and ranges in file order, so a container pair and
/// the exact pair it wraps line up whichever way each was enumerated.
fn ordered(left: &Fingerprint, right: &Fingerprint) -> ((FileId, FileId), ByteRange, ByteRange) {
    if left.file_id <= right.file_id {
        (
            (left.file_id, right.file_id),
            left.byte_range,
            right.byte_range,
        )
    } else {
        (
            (right.file_id, left.file_id),
            right.byte_range,
            left.byte_range,
        )
    }
}
