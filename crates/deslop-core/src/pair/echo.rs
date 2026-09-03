//! Exact clones a rescue pair sits around, or inside
//! ([FUSED-SHARED-SUBTREE-ECHO], [FUSED-SHARED-SUBTREE-SAME-FILE]).
//!
//! A shared-subtree rescue exists to admit a near-miss the anchor axis
//! cannot see, and the exact clones the anchor axis *did* prove answer
//! two questions about it.
//!
//! **Across files, what has already been proved?** A class shell or
//! module preamble that encloses a Merkle-equal authored function in
//! both files measures high overlap *because of that function*, and
//! admitting the container hands subsumption a wider, byte-divergent
//! view that then eats the exact one ([PIPELINE-CLUSTER-SUBSUME] prefers
//! enclosure). [`ExactClones::whole_functions_across_files`] records
//! every candidate pair that is Merkle-equal, cross-file, and a run of
//! whole authored functions on both sides, so the rescue can ask how
//! much of a container's shared mass is already claimed.
//!
//! **Inside one file, is this a copy at all?** Two methods that drifted
//! apart in one file share whole statements outright — the same
//! statement, Merkle-equal, in both — while a family of sibling
//! accessors that merely share a skeleton shares none.
//! [`ExactClones::within_one_file`] records the Merkle-equal pairs of one
//! file so the same-file rescue can measure that interior.

use std::collections::HashMap;

use crate::{
    ast::ByteRange, cluster::scope::DeclarationScopes, fingerprint::Fingerprint, state::FileId,
};

use super::CandidatePair;

/// One exact clone: the two ranges it occupies in canonical order, and
/// the nodes it claims.
#[derive(Clone, Copy)]
struct ExactClone {
    /// Range on the lower side of [`ordered`].
    first: ByteRange,
    /// Range on the higher side of [`ordered`].
    second: ByteRange,
    /// Nodes of the clone — both endpoints agree, being Merkle-equal.
    nodes: usize,
}

/// The Merkle-equal candidate pairs of a corpus, indexed by ordered file
/// pair so a rescue candidate's own file pair is one map hit.
pub(crate) struct ExactClones {
    /// Exact pairs by ordered file pair.
    by_files: HashMap<(FileId, FileId), Vec<ExactClone>>,
}

impl ExactClones {
    /// Indexes every Merkle-equal pair of `pairs` that `admits` accepts.
    fn index(
        pairs: &[CandidatePair],
        fingerprints: &[Fingerprint],
        admits: impl Fn(&Fingerprint, &Fingerprint) -> bool,
    ) -> Self {
        let mut by_files: HashMap<(FileId, FileId), Vec<ExactClone>> = HashMap::new();
        for pair in pairs {
            let (Some(left), Some(right)) =
                (fingerprints.get(pair.left), fingerprints.get(pair.right))
            else {
                continue;
            };
            if left.hash != right.hash || !admits(left, right) {
                continue;
            }
            let (key, first, second) = ordered(left, right);
            by_files.entry(key).or_default().push(ExactClone {
                first,
                second,
                nodes: left.node_count,
            });
        }
        Self { by_files }
    }

    /// The cross-file, function-aligned exact clones a container may
    /// merely be echoing ([FUSED-SHARED-SUBTREE-ECHO]).
    pub(crate) fn whole_functions_across_files<L: std::hash::BuildHasher>(
        pairs: &[CandidatePair],
        fingerprints: &[Fingerprint],
        scopes: &DeclarationScopes<'_, L>,
    ) -> Self {
        Self::index(pairs, fingerprints, |left, right| {
            left.file_id != right.file_id
                && scopes.aligned_function_run(left)
                && scopes.aligned_function_run(right)
        })
    }

    /// The exact clones of one file — the shared interior a same-file
    /// rescue is measured against ([FUSED-SHARED-SUBTREE-SAME-FILE]).
    pub(crate) fn within_one_file(pairs: &[CandidatePair], fingerprints: &[Fingerprint]) -> Self {
        Self::index(pairs, fingerprints, |left, right| {
            left.file_id == right.file_id
        })
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

    /// The most nodes any exact clone claims of the pair's shared mass,
    /// or `None` when the pair neither wraps nor sits inside one. A pair
    /// is never its own anchor: only a clone that differs from the pair
    /// on at least one side is something the pair could merely echo.
    pub(crate) fn claimed_nodes(&self, left: &Fingerprint, right: &Fingerprint) -> Option<usize> {
        let (key, first, second) = ordered(left, right);
        let sizes = (
            left.node_count.min(right.node_count),
            left.node_count.max(right.node_count),
        );
        self.others(key, first, second)?
            .filter_map(|exact| claimed_by(exact, first, second, sizes))
            .max()
    }

    /// Nodes of the largest exact clone the pair encloses, one endpoint
    /// each — the copied interior two drifted declarations still share
    /// ([FUSED-SHARED-SUBTREE-SAME-FILE]). `0` when they share none.
    pub(crate) fn enclosed_nodes(&self, left: &Fingerprint, right: &Fingerprint) -> usize {
        let (key, first, second) = ordered(left, right);
        self.others(key, first, second)
            .into_iter()
            .flatten()
            .filter(|exact| first.covers(exact.first) && second.covers(exact.second))
            .map(|exact| exact.nodes)
            .max()
            .unwrap_or(0)
    }

    /// The file pair's exact clones other than the pair itself.
    fn others(
        &self,
        key: (FileId, FileId),
        first: ByteRange,
        second: ByteRange,
    ) -> Option<impl Iterator<Item = &ExactClone>> {
        Some(
            self.by_files
                .get(&key)?
                .iter()
                .filter(move |exact| first != exact.first || second != exact.second),
        )
    }
}

/// The nodes one exact clone claims of a pair's shared mass. A container
/// on both sides — each endpoint encloses the clone — has the whole
/// clone claimed. Both endpoints inside the clone — two windows carved
/// from sibling copies of one method — can share nothing the clone does
/// not already hold, so the smaller endpoint is claimed. One endpoint
/// enclosing the clone while the other lies inside it is the same
/// bargain with nothing left over: the inside endpoint is the clone's,
/// and it is everything the container could match, so the whole pair is
/// claimed and a container is never rescued on the strength of a copy
/// it merely wraps.
fn claimed_by(
    exact: &ExactClone,
    first: ByteRange,
    second: ByteRange,
    (smaller, larger): (usize, usize),
) -> Option<usize> {
    let wraps_first = first.covers(exact.first);
    let wraps_second = second.covers(exact.second);
    let inside_first = exact.first.covers(first);
    let inside_second = exact.second.covers(second);
    if wraps_first && wraps_second {
        Some(exact.nodes)
    } else if inside_first && inside_second {
        Some(smaller)
    } else if (wraps_first && inside_second) || (inside_first && wraps_second) {
        Some(larger)
    } else {
        None
    }
}

/// The pair's file key and ranges in canonical order, so a container
/// pair and the exact pair it wraps line up whichever way each was
/// enumerated. Across files the file id orders them; inside one file the
/// candidate's index order says nothing about position, so the byte
/// offset does.
fn ordered(left: &Fingerprint, right: &Fingerprint) -> ((FileId, FileId), ByteRange, ByteRange) {
    let leads = (left.file_id, left.byte_range.start) <= (right.file_id, right.byte_range.start);
    if leads {
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
