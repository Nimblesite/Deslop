//! Deterministic survivor selection for [PIPELINE-CLUSTER-SUBSUME].

use std::cmp::Ordering;

use super::{
    super::Cluster, covers_every_file, occurrence_contains, occurrences_describe_one_location,
    strictly_encloses, Nesting,
};
use crate::fingerprint::Fingerprint;

/// Which physical cluster view survives a subsumption comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Preference {
    /// The first view survives.
    First,
    /// The second view survives.
    Second,
    /// Neither view covers the other's files, so both survive.
    Neither,
}

impl Preference {
    /// The same verdict seen from the other view's side.
    fn flipped(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
            Self::Neither => Self::Neither,
        }
    }
}

/// [PIPELINE-CLUSTER-SUBSUME] Two views are one duplication when their
/// copies pair one to one, or when one reads the other at a coarser grain.
pub(super) fn covers_same_region(first: &Cluster, second: &Cluster) -> bool {
    pair_one_to_one(&first.members, &second.members)
        || holds_evenly(&first.members, &second.members)
        || holds_evenly(&second.members, &first.members)
        || holds_shape_only(first, second)
        || holds_shape_only(second, first)
}

/// A shape-only view reports no duplication, so a wider view that holds
/// all of it, with some of it in each of its own copies, re-describes it
/// however unevenly ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
fn holds_shape_only(outer: &Cluster, inner: &Cluster) -> bool {
    !inner.kind.is_clone()
        && strictly_encloses(&outer.members, &inner.members)
        && outer.members.iter().all(|copy| {
            inner
                .members
                .iter()
                .any(|member| occurrence_contains(copy, member))
        })
}

/// One distinct copy pairs with one distinct copy by containment.
/// Election has already removed overlapping members within each file, so
/// containment pairs preserve file-and-start order.
fn pair_one_to_one(first: &[Fingerprint], second: &[Fingerprint]) -> bool {
    if first.is_empty() || first.len() != second.len() {
        return false;
    }
    let mut left: Vec<_> = first.iter().collect();
    let mut right: Vec<_> = second.iter().collect();
    left.sort_unstable_by_key(|member| (member.file_id, member.byte_range.start));
    right.sort_unstable_by_key(|member| (member.file_id, member.byte_range.start));
    left.into_iter()
        .zip(right)
        .all(|(mine, theirs)| occurrences_describe_one_location(mine, theirs))
}

/// Every `outer` copy holds the same number of `inner` copies, and each
/// `inner` copy lies in exactly one `outer` copy. A window that holds two
/// copies where another holds one cannot stand in for them.
fn holds_evenly(outer: &[Fingerprint], inner: &[Fingerprint]) -> bool {
    let mut counts = outer.iter().map(|copy| held_by(copy, inner));
    counts.next().is_some_and(|first| {
        first > 0
            && counts.all(|count| count == first)
            && inner.iter().all(|member| holders_of(member, outer) == 1)
    })
}

/// How many of `members` lie inside `copy`.
fn held_by(copy: &Fingerprint, members: &[Fingerprint]) -> usize {
    members
        .iter()
        .filter(|member| occurrence_contains(copy, member))
        .count()
}

/// How many of `copies` hold `member`.
fn holders_of(member: &Fingerprint, copies: &[Fingerprint]) -> usize {
    copies
        .iter()
        .filter(|copy| occurrence_contains(copy, member))
        .count()
}

/// Which of two views survives when they describe the same duplication,
/// or [`Preference::Neither`] when they do not. Enclosure is nominated
/// in both directions: neither argument is a nesting role, so either may
/// be the physical encloser.
pub(super) fn same_region_survivor(first: &Cluster, second: &Cluster) -> Preference {
    if !covers_same_region(first, second) {
        return Preference::Neither;
    }
    if strictly_encloses(&second.members, &first.members) {
        return preferred_view(second, first, Nesting::ProposedEncloses).flipped();
    }
    let nesting = if strictly_encloses(&first.members, &second.members) {
        Nesting::ProposedEncloses
    } else {
        Nesting::Neither
    };
    preferred_view(first, second, nesting)
}

/// Applies the exact survivor order from [PIPELINE-CLUSTER-SUBSUME].
pub(super) fn preferred_view(first: &Cluster, second: &Cluster, nesting: Nesting) -> Preference {
    match file_coverage(first, second) {
        Some(preference) => preference,
        None if nesting == Nesting::ProposedEncloses => Preference::First,
        None => compare_coverage_mass_and_id(first, second),
    }
}

/// Preserves every file before considering geometry or mass.
fn file_coverage(first: &Cluster, second: &Cluster) -> Option<Preference> {
    match (
        covers_every_file(&first.members, &second.members),
        covers_every_file(&second.members, &first.members),
    ) {
        (false, false) => Some(Preference::Neither),
        (false, true) => Some(Preference::Second),
        (true, false) => Some(Preference::First),
        (true, true) => None,
    }
}

/// Uses occurrence coverage, duplicated mass, then stable id.
fn compare_coverage_mass_and_id(first: &Cluster, second: &Cluster) -> Preference {
    match outranks(first, second) {
        Ordering::Less => Preference::Second,
        Ordering::Equal | Ordering::Greater => Preference::First,
    }
}

/// The survivor order's final tie-break — occurrence coverage, duplicated
/// mass, then stable id — as a total order: `Greater` when `first` leads.
/// Total, so it can decide a cycle ([PIPELINE-CLUSTER-SUBSUME-CYCLE]).
pub(super) fn outranks(first: &Cluster, second: &Cluster) -> Ordering {
    first
        .members
        .len()
        .cmp(&second.members.len())
        .then_with(|| first.mass.cmp(&second.mass))
        .then_with(|| second.id.cmp(&first.id))
}
