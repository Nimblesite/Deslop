//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Join overlapping identical or
//! nearly identical sibling windows into the copied run they read, before
//! the straddle rule can mistake their copied ends for padding.
//!
//! A sibling window holds at most [`crate::sibling::MAX_WINDOW_WIDTH`]
//! siblings, so a longer copied run reaches this stage as a ladder of
//! overlapping windows. Two of them are joined when their complete union
//! is one normalised shape in both files, its endpoints are sibling
//! boundaries in both parse trees, and the union itself is admitted as an
//! identical or nearly identical clone by the same pair judge every
//! fingerprinted view passes ([CLONE-KIND-FOLD]). A renamed copy joins
//! exactly as a verbatim one does; two padded windows around a shared
//! block do not, because their union fails the content floor the block
//! alone cleared ([FUSED-CONTENT-GATE]).
//!
//! Joining only adds: a run is a new view wider than any the scan
//! fingerprinted, and every window stays in place. Enclosure is the
//! kernel's verdict ([PIPELINE-CLUSTER-SUBSUME]), and a window is the
//! core, or the partner, the straddle rule reads when the run's padded
//! neighbours reach it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{duplicate_mass, identity::Unnamed, ClusterKindJudge};
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::ClusterKind,
    fingerprint::{subtree_hash, Fingerprint, HashScratch},
    sibling::window_hash_for_nodes,
    state::FileId,
};

#[cfg(test)]
#[path = "copied_runs/tests.rs"]
mod tests;

/// One two-file extent, in stable file order.
type Span = [(FileId, ByteRange); 2];

/// A span as an ordered key: both files' byte offsets.
type SpanKey = [(FileId, usize, usize); 2];

/// Adds, to `drafts`, every copied run their overlapping two-file
/// identical or nearly identical windows read, one file pair at a time.
pub(super) fn coalesce_copied_runs(
    drafts: Vec<Unnamed>,
    trees: &[NormalizedNode],
    judge: &dyn ClusterKindJudge,
) -> Vec<Unnamed> {
    if trees.is_empty() {
        return drafts;
    }
    let tree_by_file: HashMap<_, _> = trees.iter().map(|tree| (tree.file_id, tree)).collect();
    let runs: Vec<Unnamed> = groups(&drafts)
        .values()
        .flat_map(|group| group_runs(group, &drafts, &tree_by_file, judge))
        .collect();
    drafts.into_iter().chain(runs).collect()
}

/// Two-file identical or nearly identical views grouped by their file pair.
fn groups(drafts: &[Unnamed]) -> BTreeMap<(FileId, FileId), Vec<usize>> {
    let mut groups = BTreeMap::new();
    for (index, draft) in drafts.iter().enumerate() {
        if is_run_kind(draft.kind) {
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

/// The kinds a copied run can carry: verbatim, or renamed and
/// re-parameterised ([CLONE-BUCKETS-NORTH-STAR]).
fn is_run_kind(kind: ClusterKind) -> bool {
    matches!(kind, ClusterKind::Identical | ClusterKind::NearlyIdentical)
}

/// The runs one file pair's windows read: every window no other view
/// encloses grows through its overlapping neighbours while the judge
/// admits the union, and each extent reached is kept once.
fn group_runs(
    group: &[usize],
    drafts: &[Unnamed],
    trees: &HashMap<FileId, &NormalizedNode>,
    judge: &dyn ClusterKindJudge,
) -> Vec<Unnamed> {
    let views: Vec<&Unnamed> = group
        .iter()
        .filter_map(|index| drafts.get(*index))
        .collect();
    let mut known: BTreeSet<SpanKey> = views.iter().filter_map(|view| key_of(view)).collect();
    let mut runs: Vec<Unnamed> = Vec::new();
    for seed in &views {
        if enclosed(seed, &views, &runs) {
            continue;
        }
        if let Some(run) = grow(seed, &views, &runs, &known, trees, judge) {
            if key_of(&run).is_some_and(|key| known.insert(key)) {
                runs.push(run);
            }
        }
    }
    runs
}

/// Whether another view of the group, or a run already read, strictly
/// encloses `seed` in both files — then that view reads the seed's run.
fn enclosed(seed: &Unnamed, views: &[&Unnamed], runs: &[Unnamed]) -> bool {
    views
        .iter()
        .copied()
        .chain(runs)
        .any(|other| encloses_both(&other.members, &seed.members))
}

/// Extends `seed` through every neighbour whose union the judge admits,
/// or nothing when no neighbour extends it.
fn grow(
    seed: &Unnamed,
    views: &[&Unnamed],
    runs: &[Unnamed],
    known: &BTreeSet<SpanKey>,
    trees: &HashMap<FileId, &NormalizedNode>,
    judge: &dyn ClusterKindJudge,
) -> Option<Unnamed> {
    let mut run = seed.clone();
    let mut grown = false;
    while let Some(joined) = views
        .iter()
        .copied()
        .chain(runs)
        .find_map(|partner| join(&run, partner, known, trees, judge))
    {
        run = joined;
        grown = true;
    }
    grown.then_some(run)
}

/// One new view is justified only when the whole union is one copied run
/// the scan has not read already: one normalised shape in both files that
/// the judge admits as an identical or nearly identical clone.
fn join(
    run: &Unnamed,
    partner: &Unnamed,
    known: &BTreeSet<SpanKey>,
    trees: &HashMap<FileId, &NormalizedNode>,
    judge: &dyn ClusterKindJudge,
) -> Option<Unnamed> {
    let union = overlapping_union(&run.members, &partner.members)?;
    if known.contains(&span_key(union)) {
        return None;
    }
    let [(first_file, first_range), (second_file, second_range)] = union;
    let first = member(first_file, first_range, trees)?;
    let second = member(second_file, second_range, trees)?;
    if first.hash != second.hash {
        return None;
    }
    let kind = judge
        .span_kind(&first, &second)
        .filter(|kind| is_run_kind(*kind))?;
    log_join(&first, &second, kind);
    Some(joined_view(run, first, second, kind))
}

/// Records one admitted union, so a surprising extent is traceable
/// without re-running the pipeline. Byte offsets only, never source
/// text ([PRINCIPLES-LOGGING]).
fn log_join(first: &Fingerprint, second: &Fingerprint, kind: ClusterKind) {
    tracing::debug!(
        kind = ?kind,
        first_file = ?first.file_id,
        first_start = first.byte_range.start,
        first_end = first.byte_range.end,
        second_file = ?second.file_id,
        second_start = second.byte_range.start,
        second_end = second.byte_range.end,
        nodes = first.node_count.min(second.node_count),
        "copied run joined",
    );
}

/// Whether the raw bytes prove two overlapping views' complete union is
/// one copied run — the straddle rule's evidence when no sibling run
/// could be formed for that union.
pub(super) fn copied_union(
    left: &[Fingerprint],
    right: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> bool {
    overlapping_union(left, right).is_some_and(|[first, second]| {
        source_bytes(sources, first)
            .zip(source_bytes(sources, second))
            .is_some_and(|(first_bytes, second_bytes)| first_bytes == second_bytes)
    })
}

/// Both file ranges of a two-file overlap: the views pair file for file
/// and overlap partially in each.
fn overlapping_union(left: &[Fingerprint], right: &[Fingerprint]) -> Option<Span> {
    let [left_first, left_second] = paired(left)?;
    let [right_first, right_second] = paired(right)?;
    if left_first.file_id != right_first.file_id || left_second.file_id != right_second.file_id {
        return None;
    }
    let first = union(left_first.byte_range, right_first.byte_range)?;
    let second = union(left_second.byte_range, right_second.byte_range)?;
    Some([(left_first.file_id, first), (left_second.file_id, second)])
}

/// An overlapping pair must extend in both directions; containment has
/// its own subsumption rule and cannot extend the published source span.
fn union(first: ByteRange, second: ByteRange) -> Option<ByteRange> {
    first.partially_overlaps(second).then_some(ByteRange {
        start: first.start.min(second.start),
        end: first.end.max(second.end),
    })
}

/// The source span of one file range, guarded against stale byte offsets.
fn source_bytes(
    sources: &HashMap<FileId, Vec<u8>>,
    (file, range): (FileId, ByteRange),
) -> Option<&[u8]> {
    sources.get(&file)?.get(range.start..range.end)
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

/// The ordered key of a two-file view's extent, when it has one.
fn key_of(view: &Unnamed) -> Option<SpanKey> {
    let [first, second] = paired(&view.members)?;
    Some(span_key([
        (first.file_id, first.byte_range),
        (second.file_id, second.byte_range),
    ]))
}

/// A span as an ordered key.
fn span_key(span: Span) -> SpanKey {
    span.map(|(file, range)| (file, range.start, range.end))
}

/// Both occurrences of the wider view strictly contain the narrower's.
fn encloses_both(wide: &[Fingerprint], narrow: &[Fingerprint]) -> bool {
    let (Some(wide), Some(narrow)) = (paired(wide), paired(narrow)) else {
        return false;
    };
    wide.iter().zip(narrow).all(|(outer, inner)| {
        outer.file_id == inner.file_id && outer.byte_range.strictly_encloses(inner.byte_range)
    })
}

/// The union as the fingerprint the scan would have emitted for it: its
/// sibling run's normalised digest and node count, so the judge resolves
/// exactly the nodes the run holds.
fn member(
    file: FileId,
    range: ByteRange,
    trees: &HashMap<FileId, &NormalizedNode>,
) -> Option<Fingerprint> {
    let run = sibling_run(trees.get(&file)?, range)?;
    Some(Fingerprint {
        hash: run_hash(&run),
        file_id: file,
        byte_range: range,
        node_count: run
            .iter()
            .map(|node| node.subtree_node_count())
            .fold(0_usize, usize::saturating_add),
    })
}

/// An authored run is either one node or whole adjacent children of one
/// parent; a range cut through a construct cannot be joined.
fn sibling_run(tree: &NormalizedNode, range: ByteRange) -> Option<Vec<&NormalizedNode>> {
    if tree.byte_range == range {
        return Some(vec![tree]);
    }
    if let Some(run) = sibling_children(&tree.children, range) {
        return Some(run);
    }
    tree.children
        .iter()
        .filter(|child| child.byte_range.covers(range))
        .find_map(|child| sibling_run(child, range))
}

/// Whole siblings from the child starting the range to the child ending it.
fn sibling_children(children: &[NormalizedNode], range: ByteRange) -> Option<Vec<&NormalizedNode>> {
    let first = children
        .iter()
        .position(|child| child.byte_range.start == range.start)?;
    let last = children
        .iter()
        .position(|child| child.byte_range.end == range.end)?;
    let window = (first <= last)
        .then(|| children.get(first..=last))
        .flatten()?;
    Some(window.iter().collect())
}

/// The digest the scan emits for a run: a subtree's own hash for one
/// node, the sibling-window hash for several ([PIPELINE-FINGERPRINT-MERKLE]).
fn run_hash(run: &[&NormalizedNode]) -> [u8; 32] {
    match run {
        [node] => subtree_hash(node, &mut HashScratch::default()),
        _ => window_hash_for_nodes(run),
    }
}

/// The new view carries the admitted kind of its union and names its
/// verified source extent, not either old window's hash. Mass is
/// recounted from the joined parse-tree members.
fn joined_view(
    original: &Unnamed,
    first: Fingerprint,
    second: Fingerprint,
    kind: ClusterKind,
) -> Unnamed {
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
        kind,
        mass: duplicate_mass(kind, nodes, original.members.len()),
        shape_family: original.shape_family,
    }
}
