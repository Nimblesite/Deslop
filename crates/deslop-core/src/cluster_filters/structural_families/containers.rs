//! [PIPELINE-CLUSTER-ELECT-CONTAINER] — a concatenation of a family is
//! not a finding.
//!
//! A token edge can weld a structural family to the views that merely
//! *contain* it — the class holding seven shape-identical methods, the
//! sliding window spanning two of them, the file root over the lot.
//! Left in the component, those members glue every occurrence in a
//! file into one overlapping run, and the same-file overlap collapse
//! then publishes the family as one container occurrence per file:
//! seven findings become two, and the constructors, fields and imports
//! between the methods are counted as duplicated
//! (`rank_structural_only_policy`, [RANK-STRUCTURAL-ONLY]).
//!
//! Split from [`super`], which owns the weld question; this module
//! owns the container question the weld cannot express. Both are
//! [PIPELINE-CLUSTER-ELECT]'s mechanism, keyed on the digest.

use std::collections::HashMap;

use crate::{
    cluster::VERBATIM_OVERTURN_MIN_NODES, fingerprint::Fingerprint, pair::FusedCluster,
    state::FileId,
};

use super::super::family::{families_by, restrict};
use super::MIN_FAMILY_MEMBERS;

/// Fewest same-family occurrences a member must strictly enclose to be
/// a concatenation of that family
/// ([PIPELINE-CLUSTER-ELECT-CONTAINER]). One enclosed occurrence is
/// ordinary nesting — one duplication seen at two depths — which
/// [PIPELINE-CLUSTER-SUBSUME] and the same-file overlap collapse elect
/// between on discovery evidence this pass has already discarded. A
/// single enclosure still reads as a concatenation when the member's
/// own family is itself reportable and the enclosed family continues
/// past it in the member's own file ([`concatenates`],
/// [`family_overflows`]): a duplicated window padding one sibling of a
/// method row with shared scaffolding is a bounded excerpt of that
/// row, never an independent finding. A digest singleton never takes
/// this route — a lone method holding one copy of a duplicated run is
/// the ordinary nesting [PIPELINE-CLUSTER-SUBSUME] elects between
/// (`csharp-merge-readafter`). Either arm also demands the family
/// overflow the member's own family at all: a family every occurrence
/// of which sits inside the member's own family is that view's *fine
/// structure* — the `fsharp-issue-339` sibling window wholly composed
/// of two per-binding shapes — and the wider view is the finding.
const CONTAINER_MIN_ENCLOSED: usize = 2;
/// Numerator of the share of a member's bytes the enclosed family must
/// account for before the member reads as a concatenation of it.
const CONTAINER_SHARE_NUMERATOR: usize = 2;
/// Denominator of that share. Two thirds — the same boundary
/// [`crate::cluster`]'s subsumption election measured between "the
/// enclosing view is this duplication plus code that is not duplicated"
/// and "the nested view re-describes a fragment of a finding that is
/// real at full extent".
const CONTAINER_SHARE_DENOMINATOR: usize = 3;

/// [PIPELINE-CLUSTER-ELECT-CONTAINER] — the component without its
/// concatenation members, or `None` when it holds none.
///
/// A token edge can weld a structural family to the views that merely
/// *contain* it — the class holding seven shape-identical methods, the
/// sliding window spanning two of them, the file root over the lot.
/// Left in the component, those members glue every occurrence in a file
/// into one overlapping run, and the same-file overlap collapse then
/// publishes the family as one container occurrence per file: seven
/// findings become two, and the constructors, fields and imports
/// between the methods are counted as duplicated
/// (`rank_structural_only_policy`, [RANK-STRUCTURAL-ONLY]).
///
/// A member is a concatenation — not a view — of a family exactly when
/// that family both outnumbers the member's own family and supplies the
/// bulk of its bytes ([`concatenates`]). The same two measures decide
/// the cluster-level election in [PIPELINE-CLUSTER-SUBSUME]
/// (`nested_view_outnumbers`, `accounts_for_bulk`); this pass applies
/// them one stage earlier, where the weld would otherwise destroy the
/// occurrence list before subsumption ever sees it.
pub(super) fn elect_out_containers(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
) -> Option<FusedCluster> {
    let families = families_by(&fused.members, |index| {
        fingerprints.get(index).map(|member| member.hash)
    });
    let slot_of: HashMap<usize, usize> = families
        .iter()
        .enumerate()
        .flat_map(|(slot, family)| family.iter().map(move |index| (*index, slot)))
        .collect();
    let kept: Vec<usize> = fused
        .members
        .iter()
        .copied()
        .filter(|member| !is_family_container(*member, &families, &slot_of, fingerprints))
        .collect();
    if kept.len() == fused.members.len() {
        return None;
    }
    log_elected_containers(fused, &kept, fingerprints);
    Some(restrict(fused, &kept))
}

/// Records each elected container, so a surprising election is
/// traceable without re-running the pipeline. Byte offsets and node
/// counts only — never source text ([PRINCIPLES-LOGGING]).
fn log_elected_containers(fused: &FusedCluster, kept: &[usize], fingerprints: &[Fingerprint]) {
    for member in fused.members.iter().filter(|member| !kept.contains(member)) {
        let Some(container) = fingerprints.get(*member) else {
            continue;
        };
        tracing::debug!(
            file_id = ?container.file_id,
            start = container.byte_range.start,
            end = container.byte_range.end,
            node_count = container.node_count,
            "container concatenating a larger family elected out",
        );
    }
}

/// True when `member` concatenates some family that outnumbers its own
/// ([PIPELINE-CLUSTER-ELECT-CONTAINER]). A family can never contain
/// itself — equal digests mean equal node counts, which cannot nest —
/// so the family a container is measured against always survives it.
///
/// Only a family of copied *blocks* confers container status: every
/// occurrence must carry [`VERBATIM_OVERTURN_MIN_NODES`] normalised
/// nodes, the same standing floor [PIPELINE-CLUSTER-SUBSUME] demands
/// before a nested view may unseat its encloser. Without it, a run of
/// byte-equal idiom lines — four `assert` statements that are most of a
/// small test helper — would delete the umbrella that suppresses them
/// and republish the noise family (`python-issue-71`,
/// `python-issue-100-kwargs-ctor`).
fn is_family_container(
    member: usize,
    families: &[Vec<usize>],
    slot_of: &HashMap<usize, usize>,
    fingerprints: &[Fingerprint],
) -> bool {
    let own_family: &[usize] = slot_of
        .get(&member)
        .and_then(|slot| families.get(*slot))
        .map_or(&[], Vec::as_slice);
    families.iter().any(|family| {
        family.len() >= MIN_FAMILY_MEMBERS
            && family.len() > own_family.len().max(1)
            && family_has_standing(family, fingerprints)
            && concatenates(member, family, own_family, fingerprints)
    })
}

/// True when every occurrence of `family` carries at least
/// [`VERBATIM_OVERTURN_MIN_NODES`] normalised nodes — the minimum over
/// occurrences, never the sum, so a family of many tiny repeats gains
/// no standing from its cardinality.
fn family_has_standing(family: &[usize], fingerprints: &[Fingerprint]) -> bool {
    family
        .iter()
        .map(|index| fingerprints.get(*index).map_or(0, |occ| occ.node_count))
        .min()
        .is_some_and(|smallest| smallest >= VERBATIM_OVERTURN_MIN_NODES)
}

/// True when `member` strictly encloses at least
/// [`CONTAINER_MIN_ENCLOSED`] occurrences of `family` in its own file
/// and those occurrences supply at least
/// [`CONTAINER_SHARE_NUMERATOR`]/[`CONTAINER_SHARE_DENOMINATOR`] of its
/// bytes — a run of the family plus scaffolding, not a finding of its
/// own. Below either bar the member keeps its place: one enclosed
/// occurrence is a nesting, and an encloser most of whose bytes are its
/// own code is the extractable view [PIPELINE-CLUSTER-SUBSUME] must be
/// allowed to elect.
fn concatenates(
    member: usize,
    family: &[usize],
    own_family: &[usize],
    fingerprints: &[Fingerprint],
) -> bool {
    let Some(container) = fingerprints.get(member) else {
        return false;
    };
    let (enclosed_count, enclosed_bytes) = enclosed_extent(container, family, fingerprints);
    let concatenation = enclosed_count >= CONTAINER_MIN_ENCLOSED
        && family_overflows(family, own_family, None, fingerprints)
        || enclosed_count > 0
            && own_family.len() >= MIN_FAMILY_MEMBERS
            && family_overflows(family, own_family, Some(container.file_id), fingerprints);
    concatenation
        && enclosed_bytes.saturating_mul(CONTAINER_SHARE_DENOMINATOR)
            >= container
                .byte_range
                .len()
                .saturating_mul(CONTAINER_SHARE_NUMERATOR)
}

/// How many `family` occurrences `container` strictly encloses, and
/// the bytes they cover between them.
///
/// The share test in [`is_container`] is a *coverage* claim, so this is
/// the **union** extent, never the sum of the occurrence lengths. A
/// family that overlaps itself — a shape repeating at sliding offsets,
/// the same geometry [`family_overflows`] already refuses the
/// no-overflow exemption to — would otherwise claim one byte once per
/// occurrence and elect out an encloser that still holds code of its
/// own, deleting a real finding from the report. Pinned by
/// `an_encloser_is_not_a_container_of_a_family_that_overlaps_itself`.
fn enclosed_extent(
    container: &Fingerprint,
    family: &[usize],
    fingerprints: &[Fingerprint],
) -> (usize, usize) {
    let mut spans: Vec<(usize, usize)> = family
        .iter()
        .filter_map(|index| fingerprints.get(*index))
        .filter(|occurrence| strictly_inside(occurrence, container))
        .map(|occurrence| (occurrence.byte_range.start, occurrence.byte_range.end))
        .collect();
    spans.sort_unstable();
    (spans.len(), covered_bytes(&spans))
}

/// Bytes covered by `spans`, counting a byte two of them share once.
/// `spans` must be sorted by start, which is what lets one running
/// reach stand in for every span already folded.
fn covered_bytes(spans: &[(usize, usize)]) -> usize {
    spans
        .iter()
        .fold((0_usize, 0_usize), |(covered, reach), (start, end)| {
            let from = (*start).max(reach);
            (
                covered.saturating_add(end.saturating_sub(from)),
                reach.max(*end),
            )
        })
        .0
}

/// True when `family` has an occurrence no occurrence of `own_family`
/// strictly encloses — the family reaches beyond the member's kind of
/// view, so that view is a bounded excerpt of it. Restricted to
/// `within` when the caller needs the overflow in one particular file:
/// a single enclosed occurrence reads as a concatenation only when the
/// family *continues past the member in the member's own file*, which
/// is what separates padding one sibling of a method row from the #112
/// module pair whose nested run merely reaches a third file.
///
/// A view family whose own occurrences overlap one another in a file
/// forfeits the no-overflow exemption outright: sliding windows over a
/// sibling row tile the whole family between them — every sibling sits
/// inside *some* window — but the tiling counts the shared siblings
/// twice, so its coverage is not an occurrence list a report could
/// publish, and the family it tiles is never its fine structure.
fn family_overflows(
    family: &[usize],
    own_family: &[usize],
    within: Option<FileId>,
    fingerprints: &[Fingerprint],
) -> bool {
    if family_self_overlaps(own_family, fingerprints) {
        return true;
    }
    family
        .iter()
        .filter_map(|index| fingerprints.get(*index))
        .filter(|occurrence| within.map_or(true, |file| occurrence.file_id == file))
        .any(|occurrence| {
            !own_family
                .iter()
                .filter_map(|index| fingerprints.get(*index))
                .any(|own| strictly_inside(occurrence, own))
        })
}

/// True when two occurrences of one family overlap each other in one
/// file — the sliding-window tiling [`family_overflows`] refuses.
fn family_self_overlaps(family: &[usize], fingerprints: &[Fingerprint]) -> bool {
    family.iter().enumerate().any(|(position, left)| {
        family.iter().skip(position.saturating_add(1)).any(|right| {
            match (fingerprints.get(*left), fingerprints.get(*right)) {
                (Some(left), Some(right)) => {
                    left.file_id == right.file_id
                        && left.byte_range.start < right.byte_range.end
                        && right.byte_range.start < left.byte_range.end
                }
                _ => false,
            }
        })
    })
}

/// True when `inner` lies wholly inside `outer` in the same file and
/// `outer` reaches beyond it on at least one side.
fn strictly_inside(inner: &Fingerprint, outer: &Fingerprint) -> bool {
    inner.file_id == outer.file_id
        && outer.byte_range.start <= inner.byte_range.start
        && inner.byte_range.end <= outer.byte_range.end
        && inner.byte_range.len() < outer.byte_range.len()
}
