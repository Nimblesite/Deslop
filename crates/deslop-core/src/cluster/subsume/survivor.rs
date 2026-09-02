//! Deterministic survivor selection for [PIPELINE-CLUSTER-SUBSUME].

use std::cmp::Ordering;

use super::super::Cluster;
use super::{all_occurrences_paired, covers_every_file, Nesting};

/// Which physical cluster view survives a subsumption comparison.
pub(super) enum Preference {
    /// The first view survives.
    First,
    /// The second view survives.
    Second,
    /// Neither view covers the other's files, so both survive.
    Neither,
}

/// Returns `true` when both components cover the same physical regions.
pub(super) fn covers_same_region(first: &Cluster, second: &Cluster) -> bool {
    all_occurrences_paired(&first.members, &second.members)
        && all_occurrences_paired(&second.members, &first.members)
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
    match first
        .members
        .len()
        .cmp(&second.members.len())
        .then_with(|| first.mass.cmp(&second.mass))
        .then_with(|| second.id.cmp(&first.id))
    {
        Ordering::Less => Preference::Second,
        Ordering::Equal | Ordering::Greater => Preference::First,
    }
}
