//! Deterministic survivor selection for [PIPELINE-CLUSTER-SUBSUME].

use std::cmp::Ordering;

use super::{
    super::Cluster, covers_every_file, occurrences_describe_one_location, strictly_encloses,
    Nesting,
};
use crate::buckets::ClusterKind;

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

/// [PIPELINE-CLUSTER-SUBSUME] One distinct copy must pair with one distinct copy.
/// Election has already removed overlapping members within each file, so
/// containment pairs preserve file-and-start order.
pub(super) fn covers_same_region(first: &Cluster, second: &Cluster) -> bool {
    if first.members.is_empty() || first.members.len() != second.members.len() {
        return false;
    }
    let mut left: Vec<_> = first.members.iter().collect();
    let mut right: Vec<_> = second.members.iter().collect();
    left.sort_unstable_by_key(|member| (member.file_id, member.byte_range.start));
    right.sort_unstable_by_key(|member| (member.file_id, member.byte_range.start));
    left.into_iter()
        .zip(right)
        .all(|(mine, theirs)| occurrences_describe_one_location(mine, theirs))
}

/// Which of two views survives when they describe the same duplication,
/// or [`Preference::Neither`] when they do not. Enclosure is nominated
/// in both directions: neither argument is a nesting role, so either may
/// be the physical encloser.
pub(super) fn same_region_survivor(first: &Cluster, second: &Cluster) -> Preference {
    if distinct_exact_core(first, second) {
        return Preference::Neither;
    }
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

/// [PIPELINE-CLUSTER-SUBSUME-KIND] An edited copy cannot report its
/// enclosed byte-identical copy's Type I extent or classification.
fn distinct_exact_core(first: &Cluster, second: &Cluster) -> bool {
    match (first.kind, second.kind) {
        (ClusterKind::Identical, kind) if kind != ClusterKind::Identical => {
            strictly_encloses(&second.members, &first.members)
        }
        (kind, ClusterKind::Identical) if kind != ClusterKind::Identical => {
            strictly_encloses(&first.members, &second.members)
        }
        _ => false,
    }
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
