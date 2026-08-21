//! [CLONE-NOISE-VERBATIM-SUBGROUP] — partition a noise family off a
//! byte-identical copy instead of erasing both together.
//!
//! Every suppression in [`super`] guards itself with a verbatim escape
//! hatch, and every one of them states the same intent: a byte-for-byte
//! copy is real duplication and survives the filter. That guarantee was
//! written as *"at least two members differ"*, which one unrelated
//! member is enough to satisfy — so a cluster holding a proven copy
//! `A`/`A` **plus** a shape-compatible stranger `C` took the
//! suppression whole and the copy vanished from the report. A duplicate
//! that is never reported is the one defect class no reader can notice.
//!
//! The escape hatch cannot be repaired by loosening the predicate
//! alone: keeping the whole cluster would publish `C` as an occurrence
//! of a copy it is not part of, trading a false negative for a false
//! positive. The family is what has to be separated, so this pass runs
//! before signals are measured: a cluster the noise filters would
//! suppress is replaced by one cluster per byte-identical family it
//! contains, and every member outside those families is dropped. The
//! surviving cluster is then measured, bucketed, ranked and rendered
//! from exactly the occurrences it kept — no signal is inherited from
//! the members that left.
//!
//! Clusters the filters do not suppress are returned untouched, so a
//! consistently-renamed three-way clone stays one three-way clone.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{fingerprint::Fingerprint, pair::FusedCluster, state::FileId};

use super::{
    family::{families_by, restrict},
    is_noise_pattern, ParseCache,
};

/// Smallest byte-identical family worth keeping: one lone occurrence is
/// not a duplicate of anything.
const MIN_FAMILY_MEMBERS: usize = 2;

/// Replaces every fused cluster the noise filters would suppress but
/// which still contains a byte-identical family with one cluster per
/// such family, dropping the members that belong to none.
///
/// Ordering is preserved: clusters keep their input position and each
/// cluster's families are emitted in first-member order, so the pass is
/// deterministic ([PIPELINE-DETERMINISM]).
pub(crate) fn split_noise_verbatim_families<S: BuildHasher>(
    fused_clusters: Vec<FusedCluster>,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Vec<FusedCluster> {
    let cache = ParseCache::new();
    fused_clusters
        .into_iter()
        .flat_map(|fused| {
            split_one(&fused, fingerprints, sources, file_languages, &cache)
                .unwrap_or_else(|| vec![fused])
        })
        .collect()
}

/// The replacement clusters for one component, or `None` to keep it as
/// it is. `None` covers every cheap case first — a component with no
/// mixed verbatim family needs no re-parse at all — so the noise
/// filters only run on the components a split could actually change
/// ([CLONE-NOISE-REPARSE-CACHE]).
fn split_one<S: BuildHasher>(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<Vec<FusedCluster>> {
    let families = verbatim_families(&fused.members, fingerprints, sources);
    let keepable: Vec<&Vec<usize>> = families
        .iter()
        .filter(|family| family.len() >= MIN_FAMILY_MEMBERS)
        .collect();
    let covered: usize = keepable.iter().map(|family| family.len()).sum();
    if keepable.is_empty() || covered == fused.members.len() && keepable.len() == 1 {
        return None;
    }
    let members: Vec<Fingerprint> = fused
        .members
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    if members.len() != fused.members.len()
        || !is_noise_pattern(&members, sources, file_languages, cache)
    {
        return None;
    }
    Some(
        keepable
            .into_iter()
            .map(|family| restrict(fused, family))
            .collect(),
    )
}

/// Groups the component's members by the exact source bytes their
/// fingerprint covers.
fn verbatim_families(
    member_indices: &[usize],
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> Vec<Vec<usize>> {
    families_by(member_indices, |index| {
        member_text(index, fingerprints, sources)
    })
}

/// The raw source bytes one member's fingerprint covers.
fn member_text<'a>(
    index: usize,
    fingerprints: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
) -> Option<&'a [u8]> {
    let member = fingerprints.get(index)?;
    sources
        .get(&member.file_id)?
        .get(member.byte_range.start..member.byte_range.end)
}

#[cfg(test)]
mod tests;
