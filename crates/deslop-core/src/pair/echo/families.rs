//! [FUSED-SHARED-SUBTREE-SAME-FILE] Exact copies grouped by family, so
//! any two declarations can ask what they both hold.
//!
//! Candidate enumeration pairs each Merkle family as a star: every copy
//! beside the first, never two later copies with each other. Two drifted
//! methods whose shared statements also appear in an earlier sibling then
//! hold no exact pair between them, and read as sharing nothing. Merkle
//! equality is transitive, so every copy a proved pair names belongs to its
//! hash's family, and two endpoints share an interior when each encloses a
//! member of the same family.

use std::collections::HashMap;

use super::range_index::RangeIndex;
use crate::{ast::ByteRange, fingerprint::Fingerprint, state::FileId};

/// One family member's hash and node count.
type Member = ([u8; 32], usize);

/// Every endpoint of a proved exact pair, per file, ordered by range.
pub(super) struct CopyFamilies {
    /// Members by file, answered by range ([FUSED-SHARED-SUBTREE-INDEX]).
    by_file: HashMap<FileId, RangeIndex<Member>>,
}

impl CopyFamilies {
    /// Indexes both endpoints of every exact pair, each range once.
    pub(super) fn new<'a>(exact: impl Iterator<Item = (&'a Fingerprint, &'a Fingerprint)>) -> Self {
        let mut members: HashMap<FileId, HashMap<(usize, usize), Member>> = HashMap::new();
        for fingerprint in exact.flat_map(|(left, right)| [left, right]) {
            let range = fingerprint.byte_range;
            members.entry(fingerprint.file_id).or_default().extend([(
                (range.start, range.end),
                (fingerprint.hash, fingerprint.node_count),
            )]);
        }
        Self {
            by_file: members
                .into_iter()
                .map(|(file, ranges)| {
                    (
                        file,
                        RangeIndex::new(ranges.into_iter().map(entry).collect()),
                    )
                })
                .collect(),
        }
    }

    /// Nodes of the largest family with a member inside each endpoint,
    /// other than the two endpoints themselves; `0` when they share none.
    pub(super) fn enclosed_nodes(&self, left: &Fingerprint, right: &Fingerprint) -> usize {
        let held: HashMap<[u8; 32], ByteRange> = self
            .inside(left)
            .map(|(range, (hash, _))| (*hash, *range))
            .collect();
        self.inside(right)
            .filter(|(range, (hash, _))| {
                held.get(hash)
                    .is_some_and(|first| *first != left.byte_range || *range != right.byte_range)
            })
            .map(|(_, (_, nodes))| *nodes)
            .max()
            .unwrap_or(0)
    }

    /// The members lying inside one endpoint.
    fn inside<'a>(
        &'a self,
        endpoint: &'a Fingerprint,
    ) -> impl Iterator<Item = &'a (ByteRange, Member)> + 'a {
        self.by_file
            .get(&endpoint.file_id)
            .into_iter()
            .flat_map(move |members| members.related(endpoint.byte_range))
            .filter(move |(range, _)| endpoint.byte_range.covers(*range))
    }
}

/// One indexed member from its deduplicated range key.
const fn entry(((start, end), member): ((usize, usize), Member)) -> (ByteRange, Member) {
    (ByteRange { start, end }, member)
}
