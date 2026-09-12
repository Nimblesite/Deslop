//! Shared copy keys for noise partitioning and the later family escape.

use std::{collections::HashMap, hash::BuildHasher};

use super::{
    super::{
        dart,
        family::families_by,
        snippets::{collect_snippets, uniform_language, Snippet},
        ParseCache,
    },
    resolved_members,
};
use crate::{fingerprint::Fingerprint, pair::FusedCluster, state::FileId};

/// Distinct key variants keep body copies separate from whole-source copies.
#[derive(PartialEq, Eq, Hash)]
pub(super) enum CopyKey<'src> {
    /// Exact bytes of the entire reported range.
    WholeBytes(&'src [u8]),
    /// Exact ordered build bodies within complete construction-only widget classes.
    WidgetBuildBodies(Vec<Vec<u8>>),
}

/// One canonical copy proof, reused before ranking and during family suppression.
pub(super) fn copy_keys<'src, S: BuildHasher>(
    members: &[Fingerprint],
    sources: &'src HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<Vec<CopyKey<'src>>> {
    let snippets = (uniform_language(members, languages) == Some("dart"))
        .then(|| collect_snippets(members, sources, "dart", cache))
        .flatten();
    members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            member_key(
                member,
                sources,
                snippets.as_ref().and_then(|items| items.get(index)),
            )
        })
        .collect()
}

/// Selects layout bytes only for proven scaffold ranges; other members retain whole bytes.
fn member_key<'src>(
    member: &Fingerprint,
    sources: &'src HashMap<FileId, Vec<u8>>,
    snippet: Option<&Snippet<'_>>,
) -> Option<CopyKey<'src>> {
    let raw = sources
        .get(&member.file_id)?
        .get(member.byte_range.start..member.byte_range.end)?;
    Some(
        snippet
            .and_then(dart::widget_layout_key)
            .map_or(CopyKey::WholeBytes(raw), CopyKey::WidgetBuildBodies),
    )
}

/// Group component indices with the same proof; later restriction retains only their edges.
pub(super) fn families_for<S: BuildHasher>(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<Vec<Vec<usize>>> {
    let members = resolved_members(fused, fingerprints)?;
    if uniform_language(&members, languages) != Some("dart") {
        return Some(verbatim_families(&fused.members, fingerprints, sources));
    }
    let keys = copy_keys(&members, sources, languages, cache)?;
    let keyed: HashMap<_, _> = fused.members.iter().copied().zip(keys).collect();
    Some(families_by(&fused.members, |index| keyed.get(&index)))
}

/// Groups the component's members by the exact source bytes their
/// fingerprint covers ([CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES]) —
/// no normalised comparison and no trivia tolerance, so a family whose
/// members differ in one byte is not a verbatim family at all.
pub(super) fn verbatim_families(
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
