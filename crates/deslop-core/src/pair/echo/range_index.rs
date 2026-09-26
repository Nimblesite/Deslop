//! [FUSED-SHARED-SUBTREE-INDEX] Ranges ordered by start byte, answered
//! by query rather than by scan.
//!
//! An exact-clone question asks which recorded ranges lie inside a
//! declaration's range, or wrap it. Ordered by start byte, the ranges
//! inside the query form one run that begins where the query begins,
//! and the ranges wrapping it all start earlier and still reach its
//! end. A running maximum of the ends says how far back that second
//! walk has to go: once no earlier range reaches the query's end, none
//! before it can either.

use crate::ast::ByteRange;

/// Payload-carrying ranges in ascending start order.
pub(super) struct RangeIndex<T> {
    /// Every recorded range with its payload, ascending by start then
    /// end.
    entries: Vec<(ByteRange, T)>,
    /// For each position, the furthest end among the entries at or
    /// before it — how far the prefix reaches.
    reach: Vec<usize>,
}

impl<T> RangeIndex<T> {
    /// Orders `entries` for range queries.
    pub(super) fn new(mut entries: Vec<(ByteRange, T)>) -> Self {
        entries.sort_by_key(|(range, _)| (range.start, range.end));
        let reach = entries
            .iter()
            .scan(0, |furthest, (range, _)| {
                *furthest = (*furthest).max(range.end);
                Some(*furthest)
            })
            .collect();
        Self { entries, reach }
    }

    /// Every entry whose range lies inside `query` or covers it, in no
    /// particular order, visiting none of the entries that do neither.
    pub(super) fn related(&self, query: ByteRange) -> impl Iterator<Item = &(ByteRange, T)> + '_ {
        let split = self
            .entries
            .partition_point(|(range, _)| range.start < query.start);
        let (before, from) = self.entries.split_at_checked(split).unwrap_or((&[], &[]));
        let reach = self.reach.get(..split).unwrap_or(&[]);
        let inside = from
            .iter()
            .take_while(move |(range, _)| range.start <= query.end)
            .filter(move |(range, _)| query.covers(*range) || range.covers(query));
        let around = before
            .iter()
            .zip(reach)
            .rev()
            .take_while(move |(_, furthest)| **furthest >= query.end)
            .map(|(entry, _)| entry)
            .filter(move |(range, _)| range.covers(query));
        inside.chain(around)
    }
}
