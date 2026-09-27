//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Join overlapping exact sibling
//! windows before the straddle rule can mistake their copied ends for padding.

use std::collections::{BTreeMap, HashMap};

use super::{duplicate_mass, identity::Unnamed};
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::ClusterKind,
    fingerprint::Fingerprint,
    state::FileId,
};

#[cfg(test)]
#[path = "exact_runs/tests.rs"]
mod tests;

/// Extends two-file byte-identical windows only when their whole union is
/// byte-identical and has sibling boundaries in both parse trees.
pub(super) fn coalesce_exact_runs(
    drafts: Vec<Unnamed>,
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>>,
) -> Vec<Unnamed> {
    if sources.is_empty() {
        return drafts;
    }
    let tree_by_file: HashMap<_, _> = trees.iter().map(|tree| (tree.file_id, tree)).collect();
    let groups = groups(&drafts);
    let originals = original_exact_views(&groups, &drafts);
    let mut slots: BTreeMap<usize, Unnamed> = drafts.into_iter().enumerate().collect();
    for group in groups.values() {
        coalesce_group(group, &mut slots, &tree_by_file, sources);
        restore_originals(group, &originals, &mut slots);
        discard_covered_views(group, &originals, &mut slots);
    }
    slots.into_values().collect()
}

/// Retains original fingerprints so a joined span can keep their stable ID.
fn original_exact_views(
    groups: &BTreeMap<(FileId, FileId), Vec<usize>>,
    drafts: &[Unnamed],
) -> BTreeMap<usize, Unnamed> {
    groups
        .values()
        .flatten()
        .filter_map(|index| drafts.get(*index).cloned().map(|draft| (*index, draft)))
        .collect()
}

/// Exact two-file views grouped by their file pair.
fn groups(drafts: &[Unnamed]) -> BTreeMap<(FileId, FileId), Vec<usize>> {
    let mut groups = BTreeMap::new();
    for (index, draft) in drafts.iter().enumerate() {
        if draft.kind == ClusterKind::Identical {
            if let Some([first, second]) = paired(&draft.members) {
                groups
                    .entry((first.file_id, second.file_id))
                    .or_insert_with(Vec::new)
                    .push(index);
            }
        }
    }
    groups
}

/// Keeps extending each copied run while another view proves more bytes.
fn coalesce_group(
    group: &[usize],
    slots: &mut BTreeMap<usize, Unnamed>,
    trees: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>>,
) {
    for &first in group {
        while let Some((second, joined)) = next_join(first, group, slots, trees, sources) {
            let _prior = slots.insert(first, joined);
            let _removed = slots.remove(&second);
        }
    }
}

/// Finds another overlapping view whose full union has source proof.
fn next_join(
    first: usize,
    group: &[usize],
    slots: &BTreeMap<usize, Unnamed>,
    trees: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>>,
) -> Option<(usize, Unnamed)> {
    group.iter().copied().find_map(|second| {
        let left = slots.get(&first)?;
        let right = slots.get(&second)?;
        let joined = (first != second)
            .then(|| join(left, right, trees, sources))
            .flatten()?;
        Some((second, joined))
    })
}

/// Restores a genuine original fingerprint when a merged run has its span.
fn restore_originals(
    group: &[usize],
    originals: &BTreeMap<usize, Unnamed>,
    slots: &mut BTreeMap<usize, Unnamed>,
) {
    for &index in group {
        if let Some(replacement) = original_replacement(index, group, originals, slots) {
            discard_same_span(group, index, &replacement, slots);
        }
    }
}

/// Use an authored fingerprint only when joining changed this slot's extent.
fn original_replacement(
    index: usize,
    group: &[usize],
    originals: &BTreeMap<usize, Unnamed>,
    slots: &BTreeMap<usize, Unnamed>,
) -> Option<Unnamed> {
    let merged = slots.get(&index)?;
    if originals.get(&index)?.members == merged.members {
        return None;
    }
    original_for_range(group, originals, &merged.members)
}

/// Finds an authored view of the merged bytes, if one was already admitted.
fn original_for_range(
    group: &[usize],
    originals: &BTreeMap<usize, Unnamed>,
    members: &[Fingerprint],
) -> Option<Unnamed> {
    group
        .iter()
        .filter_map(|index| originals.get(index))
        .find(|view| same_spans(&view.members, members))
        .cloned()
}

/// Keeps one original view, dropping its redundant synthetic counterpart.
fn discard_same_span(
    group: &[usize],
    keep: usize,
    original: &Unnamed,
    slots: &mut BTreeMap<usize, Unnamed>,
) {
    let duplicates = duplicate_positions(group, keep, original, slots);
    let _synthetic = slots.insert(keep, original.clone());
    for index in duplicates {
        let _duplicate = slots.remove(&index);
    }
}

/// Other live views of exactly the same physical two-file extent.
fn duplicate_positions(
    group: &[usize],
    keep: usize,
    original: &Unnamed,
    slots: &BTreeMap<usize, Unnamed>,
) -> Vec<usize> {
    group
        .iter()
        .copied()
        .filter(|index| {
            *index != keep
                && slots
                    .get(index)
                    .is_some_and(|view| same_spans(&view.members, &original.members))
        })
        .collect()
}

/// Both occurrences cover exactly the same bytes in the same files.
fn same_spans(first: &[Fingerprint], second: &[Fingerprint]) -> bool {
    let (Some(first), Some(second)) = (paired(first), paired(second)) else {
        return false;
    };
    first
        .iter()
        .zip(second)
        .all(|(left, right)| left.file_id == right.file_id && left.byte_range == right.byte_range)
}

/// A newly verified exact run replaces narrower readings of those bytes.
fn discard_covered_views(
    group: &[usize],
    originals: &BTreeMap<usize, Unnamed>,
    slots: &mut BTreeMap<usize, Unnamed>,
) {
    for members in synthetic_members(group, originals, slots) {
        for index in covered_positions(group, &members, slots) {
            let _covered = slots.remove(&index);
        }
    }
}

/// Newly joined views, excluding candidates restored to authored fingerprints.
fn synthetic_members(
    group: &[usize],
    originals: &BTreeMap<usize, Unnamed>,
    slots: &BTreeMap<usize, Unnamed>,
) -> Vec<Vec<Fingerprint>> {
    group
        .iter()
        .filter_map(|index| {
            let view = slots.get(index)?;
            (originals.get(index)?.members != view.members).then(|| view.members.clone())
        })
        .collect()
}

/// Narrower exact views that one verified run makes redundant.
fn covered_positions(
    group: &[usize],
    members: &[Fingerprint],
    slots: &BTreeMap<usize, Unnamed>,
) -> Vec<usize> {
    group
        .iter()
        .copied()
        .filter(|index| {
            slots
                .get(index)
                .is_some_and(|view| encloses_both(members, &view.members))
        })
        .collect()
}

/// Both occurrences of the larger exact run strictly contain the smaller.
fn encloses_both(wide: &[Fingerprint], narrow: &[Fingerprint]) -> bool {
    let (Some(wide), Some(narrow)) = (paired(wide), paired(narrow)) else {
        return false;
    };
    wide.iter().zip(narrow).all(|(outer, inner)| {
        outer.file_id == inner.file_id && outer.byte_range.strictly_encloses(inner.byte_range)
    })
}

/// Members in stable file order, with exactly one occurrence in each file.
fn paired(members: &[Fingerprint]) -> Option<[&Fingerprint; 2]> {
    let [first, second] = members else {
        return None;
    };
    match first.file_id.cmp(&second.file_id) {
        std::cmp::Ordering::Less => Some([first, second]),
        std::cmp::Ordering::Greater => Some([second, first]),
        std::cmp::Ordering::Equal => None,
    }
}

/// One new view is justified only by the raw content of both full unions.
fn join(
    left: &Unnamed,
    right: &Unnamed,
    trees: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>>,
) -> Option<Unnamed> {
    let [(first_file, first_range), (second_file, second_range)] =
        exact_union(&left.members, &right.members, sources)?;
    let first = member(first_file, first_range, trees, sources)?;
    let second = member(second_file, second_range, trees, sources)?;
    Some(joined_view(left, first, second))
}

/// Whether the two complete overlapping views prove a longer copied run.
pub(super) fn copied_union(
    left: &[Fingerprint],
    right: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> bool {
    exact_union(left, right, sources).is_some()
}

/// Both file ranges of an overlap whose whole union has identical bytes.
fn exact_union(
    left: &[Fingerprint],
    right: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> Option<[(FileId, ByteRange); 2]> {
    let [left_first, left_second] = paired(left)?;
    let [right_first, right_second] = paired(right)?;
    if left_first.file_id != right_first.file_id || left_second.file_id != right_second.file_id {
        return None;
    }
    let first = union(left_first.byte_range, right_first.byte_range)?;
    let second = union(left_second.byte_range, right_second.byte_range)?;
    let first_bytes = source_bytes(sources, left_first.file_id, first)?;
    let second_bytes = source_bytes(sources, left_second.file_id, second)?;
    (first_bytes == second_bytes)
        .then_some([(left_first.file_id, first), (left_second.file_id, second)])
}

/// The source span of one member, guarded against stale byte offsets.
fn source_bytes(
    sources: &HashMap<FileId, Vec<u8>>,
    file: FileId,
    range: ByteRange,
) -> Option<&[u8]> {
    sources.get(&file)?.get(range.start..range.end)
}

/// An overlapping pair must extend in both directions; containment has
/// its own subsumption rule and cannot extend the published source span.
fn union(first: ByteRange, second: ByteRange) -> Option<ByteRange> {
    first.partially_overlaps(second).then_some(ByteRange {
        start: first.start.min(second.start),
        end: first.end.max(second.end),
    })
}

/// Reads the exact source and counts the AST nodes in its sibling run.
fn member(
    file: FileId,
    range: ByteRange,
    trees: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>>,
) -> Option<Fingerprint> {
    let bytes = source_bytes(sources, file, range)?;
    let nodes = sibling_node_count(trees.get(&file)?, range)?;
    Some(Fingerprint {
        hash: *blake3::hash(bytes).as_bytes(),
        file_id: file,
        byte_range: range,
        node_count: nodes,
    })
}

/// An authored run is either one node or whole adjacent children of one
/// parent; a range cut through a construct cannot be joined.
fn sibling_node_count(tree: &NormalizedNode, range: ByteRange) -> Option<usize> {
    if tree.byte_range == range {
        return Some(tree.subtree_node_count());
    }
    if let Some(nodes) = sibling_children_count(&tree.children, range) {
        return Some(nodes);
    }
    tree.children
        .iter()
        .filter(|child| child.byte_range.covers(range))
        .find_map(|child| sibling_node_count(child, range))
}

/// Count whole siblings between the exact starting and ending child.
fn sibling_children_count(children: &[NormalizedNode], range: ByteRange) -> Option<usize> {
    let first = children
        .iter()
        .position(|child| child.byte_range.start == range.start)?;
    let last = children
        .iter()
        .position(|child| child.byte_range.end == range.end)?;
    let window = (first <= last)
        .then(|| children.get(first..=last))
        .flatten()?;
    Some(window.iter().map(NormalizedNode::subtree_node_count).sum())
}

/// The new fingerprint names its verified source extent, not either old
/// window's hash. Mass is recounted from the joined parse-tree members.
fn joined_view(original: &Unnamed, first: Fingerprint, second: Fingerprint) -> Unnamed {
    let nodes = first.node_count.min(second.node_count);
    let members = if original
        .members
        .first()
        .is_some_and(|member| member.file_id == first.file_id)
    {
        vec![first, second]
    } else {
        vec![second, first]
    };
    Unnamed {
        members,
        kind: ClusterKind::Identical,
        mass: duplicate_mass(ClusterKind::Identical, nodes, original.members.len()),
        shape_family: original.shape_family,
    }
}
