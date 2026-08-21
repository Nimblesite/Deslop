//! [PIPELINE-CLUSTER-ELECT] — a token bridge may not fuse two distinct
//! structural families into one cluster that then reports neither.
//!
//! Clusters are the connected components of the candidate-pair graph
//! ([`crate::pair::cluster_by_transitive_closure`]), and that graph
//! carries two kinds of edge. A structural edge joins members whose
//! normalised subtrees hash to the same value — the same code, up to the
//! identifier and literal renames normalisation erases. A token edge
//! joins members an LSH band collision found merely *similar*. Transitive
//! closure treats them alike, so one token edge is enough to weld two
//! structural families into a single component.
//!
//! The component that results is not a clone of anything. Its members do
//! not agree, so the content stage measures a low `agreement` and a low
//! `rename_consistency` over the union, the cluster buckets down to
//! `loosely_similar`, and report policy hides it. Both real families are
//! then lost *to the presence of each other*, and the loss grows with the
//! corpus: the more code a scan reaches, the more token bridges it finds.
//! `csharp-mcp` is the smallest case that shows it — a summing loop
//! copied into two files and a multiplying loop copied into two more,
//! each pair reported `nearly_identical` on its own, both gone when the
//! four files are scanned together.
//!
//! Splitting is safe in the direction that matters. Two distinct hashes
//! mean two distinct normalised subtrees: renaming every identifier and
//! changing every literal cannot produce them, because normalisation
//! collapses exactly those. A component holding two families of two is
//! therefore a merge, not a four-way clone, and reporting the families
//! separately publishes strictly more true duplication than reporting
//! their union — which is published as nothing at all.
//!
//! A component with one structural family and a fringe of near-misses is
//! left alone: that is an ordinary Type-3 cluster, and its fringe members
//! are occurrences a reader wants. Nesting is left alone too. Families
//! that cover one another's bytes in every file are one duplication seen
//! at two depths, not a weld, and [PIPELINE-CLUSTER-SUBSUME] and the
//! same-file overlap collapse elect between those views on evidence this
//! pass cannot see. So the families are first merged into the *regions*
//! they cover, and only a component spanning **two or more** regions is
//! split.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{fingerprint::Fingerprint, pair::FusedCluster, state::FileId};

use super::family::{families_by, restrict};

/// Smallest family worth reporting on its own: one lone occurrence is
/// not a duplicate of anything.
const MIN_FAMILY_MEMBERS: usize = 2;

/// Fewest regions that make a component a merge rather than one
/// duplication seen at several depths.
const MIN_REGIONS_TO_SPLIT: usize = 2;

/// Replaces every fused component that welds two or more separate regions
/// of code with one component per region, dropping the members that belong
/// to no reportable family.
///
/// Components covering a single region are returned untouched, so an
/// ordinary Type-3 cluster keeps every occurrence it had.
pub(crate) fn split_structural_families<S: BuildHasher>(
    fused_clusters: Vec<FusedCluster>,
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Vec<FusedCluster> {
    fused_clusters
        .into_iter()
        .flat_map(|fused| {
            split_one(&fused, fingerprints, file_languages).unwrap_or_else(|| vec![fused])
        })
        .collect()
}

/// The replacement components for one cluster, or `None` to keep it as
/// it is.
fn split_one<S: BuildHasher>(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Option<Vec<FusedCluster>> {
    if !speaks_one_language(&fused.members, fingerprints, file_languages) {
        return None;
    }
    let families = families_by(&fused.members, |index| {
        fingerprints.get(index).map(|member| member.hash)
    });
    let reportable: Vec<&[usize]> = families
        .iter()
        .map(Vec::as_slice)
        .filter(|family| family.len() >= MIN_FAMILY_MEMBERS)
        .collect();
    let regions = regions(&reportable, fingerprints);
    (regions.len() >= MIN_REGIONS_TO_SPLIT).then(|| {
        regions
            .iter()
            .map(|region| restrict(fused, region))
            .collect()
    })
}

/// True when every member of the component was parsed from the same
/// language.
///
/// The digest premise holds inside one language only. Two normalised
/// subtrees of one grammar differ because the code differs; two subtrees
/// of *different* grammars differ because the grammars do, and nothing
/// follows about whether either is a copy of the other. A component
/// spanning languages exists only because [CONFIG-CROSS-LANGUAGE] was
/// opted into, and splitting it into one cluster per language deletes
/// precisely the finding that opt-in asks for. A member whose language
/// cannot be resolved is unresolved, not agreeing: one of them is enough
/// to hold the whole component back, because an unknown grammar is exactly
/// the case this guard exists to refuse.
fn speaks_one_language<S: BuildHasher>(
    members: &[usize],
    fingerprints: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let mut languages = members.iter().map(|index| {
        fingerprints
            .get(*index)
            .and_then(|member| file_languages.get(&member.file_id))
    });
    languages
        .next()
        .flatten()
        .is_some_and(|first| languages.all(|language| language == Some(first)))
}

/// Partitions `families` into the regions of source they cover, merging
/// every family that describes the same region as another.
///
/// Each region's members come out in the component's own ascending order,
/// so the split is deterministic ([PIPELINE-DETERMINISM]).
fn regions(families: &[&[usize]], fingerprints: &[Fingerprint]) -> Vec<Vec<usize>> {
    families
        .iter()
        .fold(Vec::new(), |groups, family| {
            absorb(groups, family, fingerprints)
        })
        .into_iter()
        .map(|mut region| {
            region.sort_unstable();
            region
        })
        .collect()
}

/// Adds `family` to `groups`, merged with every group already covering the
/// same region.
fn absorb(
    groups: Vec<Vec<usize>>,
    family: &[usize],
    fingerprints: &[Fingerprint],
) -> Vec<Vec<usize>> {
    let (same, mut apart): (Vec<Vec<usize>>, Vec<Vec<usize>>) = groups
        .into_iter()
        .partition(|group| covers_one_region(group, family, fingerprints));
    let mut merged: Vec<usize> = same.into_iter().flatten().collect();
    merged.extend_from_slice(family);
    apart.push(merged);
    apart
}

/// True when two member sets describe one region of source seen at two
/// depths: every occurrence of each shares bytes with some occurrence of
/// the other.
///
/// Mutual coverage is what separates a weld from a nesting. A copied
/// method and the statement run inside it cover the same bytes in the same
/// files, and electing between those views belongs to
/// [PIPELINE-CLUSTER-SUBSUME] and the same-file overlap collapse, which
/// read the discovery evidence this pass has already discarded. Splitting
/// them apart here publishes both, showing one duplicate as two findings
/// and counting its lines twice — and it strands the byte-proven view
/// against a wider near-miss that outranks it across clusters.
///
/// One-way coverage is not a nesting. A shallow shape duplicated across
/// four files encloses a two-file clone in two of them and covers code the
/// clone never reaches in the other two, so neither is a view of the other
/// and both are real findings.
fn covers_one_region(left: &[usize], right: &[usize], fingerprints: &[Fingerprint]) -> bool {
    covers(left, right, fingerprints) && covers(right, left, fingerprints)
}

/// True when every member of `inner` shares bytes with some member of
/// `outer`.
fn covers(inner: &[usize], outer: &[usize], fingerprints: &[Fingerprint]) -> bool {
    inner.iter().all(|member| {
        outer
            .iter()
            .any(|other| shares_bytes(*member, *other, fingerprints))
    })
}

/// True when two members cover overlapping bytes of one file. An
/// unresolvable member counts as overlapping, so a missing fingerprint can
/// only ever stop a split, never cause one.
fn shares_bytes(left: usize, right: usize, fingerprints: &[Fingerprint]) -> bool {
    let (Some(left), Some(right)) = (fingerprints.get(left), fingerprints.get(right)) else {
        return true;
    };
    left.file_id == right.file_id
        && left.byte_range.start < right.byte_range.end
        && right.byte_range.start < left.byte_range.end
}

#[cfg(test)]
mod tests;
