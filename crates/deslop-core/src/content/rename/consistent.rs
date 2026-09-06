//! [FUSED-CONTENT-GATE-RENAME] The rename test: the Type-2 definition
//! itself, asked of a pair before any anchor-mass pricing.

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

use crate::state::FileId;

use super::{
    super::frontier::{frontiers_aligned, leaf_bytes, population, MemberContent, Population},
    literal_echoes, rename_mapping,
};

/// Whether the pair is one code written twice under a consistent
/// renaming: the frontiers align, no substituted identifier position
/// contradicts the modal bijection, at least one substitution is
/// corroborated, and the copy keeps at least as many names as it
/// renames. A substitution is corroborated by the same symbol renamed
/// the same way twice, by a literal echoing it, or by sibling symbols
/// renamed by the same transformation ([`transformation_siblings`]).
/// A literal that drifted with the rename is not evidence *against*
/// the copy, so it cannot refuse it. One name mapped two ways still
/// does; a family whose every name appears once proves no rename; and
/// a family that renames more than it keeps is a different vocabulary
/// over one shape — the scaffolding the gate exists to refuse, where
/// nothing outside the substitution vouches.
pub(in crate::content) fn pair_rename_is_consistent<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> bool {
    if !frontiers_aligned(canonical, member) {
        return false;
    }
    let mapping = rename_mapping(
        &population(&canonical.keys, &member.keys, Population::Identifier),
        &corroboration(canonical, member, sources),
    );
    tracing::trace!(
        constrained = mapping.constrained,
        explained = mapping.explained,
        renamed = mapping.renamed,
        corroborated = mapping.corroborated,
        identity = mapping.identity,
        "rename consistency mapping"
    );
    mapping.constrained == mapping.explained
        && mapping.corroborated > 0
        && mapping.identity >= mapping.renamed
}

/// Corroboration per substituted identifier pair: literal echoes of the
/// substitution plus its transformation siblings, both counted toward
/// the repetition [`rename_mapping`] demands.
fn corroboration<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> BTreeMap<(u64, u64), usize> {
    let mut counts = literal_echoes(canonical, member, sources).per_substitution;
    for (keys, siblings) in transformation_siblings(canonical, member, sources) {
        let slot = counts.entry(keys).or_insert(0_usize);
        *slot = slot.saturating_add(siblings);
    }
    counts
}

/// Per substituted identifier pair, how many *other* distinct
/// substitutions of the pair apply the same transformation — the bytes
/// left once the shared prefix and suffix are stripped.
/// `STREAM_CRYPTO_PROTO_TLSv1_0 -> CURL_SSLVERSION_TLSv1_0` and its
/// `_1` and `_2` siblings are one rename of a prefix seen three times,
/// and corroborate one another exactly as a repeated symbol would.
fn transformation_siblings<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> BTreeMap<(u64, u64), usize> {
    let cores = substitution_cores(canonical, member, sources);
    let mut by_core: BTreeMap<(Vec<u8>, Vec<u8>), usize> = BTreeMap::new();
    for core in cores.values() {
        let slot = by_core.entry(core.clone()).or_insert(0_usize);
        *slot = slot.saturating_add(1);
    }
    cores
        .into_iter()
        .map(|(keys, core)| {
            let siblings = by_core.get(&core).copied().unwrap_or_default();
            (keys, siblings.saturating_sub(1))
        })
        .collect()
}

/// The distinct substituted identifier pairs of the aligned frontiers,
/// each with the transformation its raw bytes apply.
fn substitution_cores<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> BTreeMap<(u64, u64), (Vec<u8>, Vec<u8>)> {
    let mut cores = BTreeMap::new();
    for (index, (left, right)) in canonical.keys.iter().zip(member.keys.iter()).enumerate() {
        let identifiers =
            left.population == Population::Identifier && right.population == Population::Identifier;
        if !identifiers || left.key == right.key || cores.contains_key(&(left.key, right.key)) {
            continue;
        }
        let bytes = leaf_bytes(canonical, index, sources).zip(leaf_bytes(member, index, sources));
        if let Some((from, to)) = bytes {
            let _previous = cores.insert((left.key, right.key), strip_common_affixes(from, to));
        }
    }
    cores
}

/// The two byte strings with their longest common prefix and longest
/// common suffix removed: what the substitution actually replaced.
fn strip_common_affixes(left: &[u8], right: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let prefix = left
        .iter()
        .zip(right)
        .take_while(|(from, to)| from == to)
        .count();
    let suffix = left
        .iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(from, to)| from == to)
        .count()
        .min(left.len().saturating_sub(prefix))
        .min(right.len().saturating_sub(prefix));
    let core = |bytes: &[u8]| {
        bytes
            .get(prefix..bytes.len().saturating_sub(suffix))
            .unwrap_or_default()
            .to_vec()
    };
    (core(left), core(right))
}
