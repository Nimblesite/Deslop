//! Edited copies of a data table ([CLONE-NOISE-DATA-ROWS]): split out of a
//! convicted component, and let through the later suppression check.

use std::{collections::HashMap, hash::BuildHasher};

use super::{distinct_locations, resolved_members, MIN_FAMILY_OCCURRENCES};
use crate::{
    cluster_filters::{
        collect_snippets,
        data_rows::{edited_copy_families, members_are_copies},
        family::restrict,
        is_noise_pattern,
        snippets::ParseCache,
        uniform_language, NoiseFilter,
    },
    fingerprint::Fingerprint,
    pair::FusedCluster,
    state::FileId,
};

/// Whether these members are nothing but edited copies of one another: the
/// cluster [`split_edited_copies`] publishes ([CLONE-NOISE-DATA-ROWS]).
pub(super) fn is_edited_copy<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    uniform_language(members, file_languages)
        .and_then(|language| collect_snippets(members, sources, language, cache))
        .is_some_and(|snippets| members_are_copies(&snippets))
}

/// Split the edited copies out of a component the data-row filter convicts
/// ([CLONE-NOISE-DATA-ROWS]), or `None` when it is not convicted or holds none.
pub(super) fn split_edited_copies<S: BuildHasher>(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<Vec<FusedCluster>> {
    let members = resolved_members(fused, fingerprints)?;
    let filter = is_noise_pattern(&members, sources, file_languages, cache)?;
    if filter != NoiseFilter::DataRows {
        return None;
    }
    let language = uniform_language(&members, file_languages)?;
    let snippets = collect_snippets(&members, sources, language, cache)?;
    let families: Vec<FusedCluster> = edited_copy_families(&snippets)
        .iter()
        .map(|family| {
            family
                .iter()
                .filter_map(|position| fused.members.get(*position).copied())
                .collect::<Vec<usize>>()
        })
        .filter(|family| distinct_locations(family, fingerprints) >= MIN_FAMILY_OCCURRENCES)
        .map(|family| restrict(fused, &family))
        .collect();
    (!families.is_empty()).then_some(families)
}
