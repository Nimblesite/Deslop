//! [FUSED-SHARED-SUBTREE-CORE] The aligned core of two endpoints: the
//! code they provably share, paired position for position.
//!
//! An ordered tree alignment maps a node to a node only when their
//! parents are mapped and their order among siblings agrees — the Tai
//! mapping the Zhang–Shasha distance is optimal over. The distance is
//! all that alignment exposes, so this module builds a mapping with the
//! same two properties from the top down. Among two sibling sequences,
//! Merkle-equal subtrees are paired by the heaviest common subsequence —
//! order-preserving, and weighted by node mass, so a statement moved
//! across a loop is paired with itself and the loop it crossed stays
//! paired: a unit-weight subsequence ties between two three-node `let`s
//! and one twenty-node `for`, and a greedy cursor walk lost the whole
//! `for` of the reordered `ledger` pair outright, then paired a
//! nine-byte `entry * 3` with an unrelated `… * 2` and read a copy as
//! 0.6 agreement. The siblings left over on both sides are paired by
//! kind, again in order, and the pairing descends into their children: a
//! method that differs by one inserted statement is not one unmatched
//! subtree but its aligned body, down to a `return` two nodes wide. A
//! leaf paired with a leaf is a frontier position paired with a frontier
//! position — which is exactly what the content gate reads.
//!
//! Every pair emitted is Merkle-equal or a same-kind leaf pair, so the
//! two content frontiers over the core align position for position, and
//! nothing outside the mapping is ever counted as agreement.

use std::{collections::HashMap, sync::Arc};

use super::{endpoint_key, OverlapMeasurer, ENDPOINT_VIEW_MEMO_MAX};
use crate::{
    ast::NormalizedNode,
    buckets::CONTENT_SUPPORT_FLOOR,
    content::ContentEvidence,
    fingerprint::{collect_fingerprints, Fingerprint},
    state::FileId,
    tokens::resolve_range_nodes,
};

/// Core pairing unit tests ([FUSED-SHARED-SUBTREE-CORE]).
#[cfg(test)]
mod tests;

/// [FUSED-SHARED-SUBTREE-CORE] The verdict on one aligned core.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CoreVerdict {
    /// Nodes the core's paired spans carry, each pair at its smaller side.
    pub(crate) nodes: usize,
    /// The gate's measurement over the core; unmeasured when the core
    /// is below the node floor, because nothing that small is measured.
    pub(crate) evidence: ContentEvidence,
    /// Whether the code the two endpoints share is a copy.
    pub(crate) copy: bool,
}

/// [FUSED-SHARED-SUBTREE-CORE] Judges an aligned core — the one reading
/// the rescue and an explicit comparison share.
///
/// A rescued pair is a Type-1 or Type-2 clone with an edit, and the core
/// is that clone, so the core must first be a clone the scan would
/// report on its own: at least `floor` nodes, the scan's `min_nodes` —
/// the size below which it reports no subtree at all. At the default
/// floor the two Playwright lines every browser test starts with, plus
/// one identifier, are twenty nodes measuring exactly the content floor;
/// they are not a clone the scan reports, and no pair is admitted on
/// them. A core that clears the node floor is measured with the gate's
/// own axes (`measure`) and is a copy when that measurement clears
/// [`CONTENT_SUPPORT_FLOOR`] or is a contradiction-free rename.
pub(crate) fn judge_core(
    core: &[(Fingerprint, Fingerprint)],
    floor: usize,
    measure: impl FnOnce() -> ContentEvidence,
) -> CoreVerdict {
    let nodes = core_node_count(core);
    if nodes < floor {
        return CoreVerdict {
            nodes,
            evidence: ContentEvidence::unmeasured(),
            copy: false,
        };
    }
    let evidence = measure();
    CoreVerdict {
        nodes,
        evidence,
        copy: evidence.clears(CONTENT_SUPPORT_FLOOR),
    }
}

/// The core's node mass: each paired span at its smaller side, summed.
fn core_node_count(core: &[(Fingerprint, Fingerprint)]) -> usize {
    core.iter()
        .map(|(span, partner)| span.node_count.min(partner.node_count))
        .sum()
}

/// Every subtree is emitted, leaves included: the core pairs down to
/// single frontier positions.
const EVERY_SUBTREE: usize = 1;

/// The weight of one same-kind pairing among leftovers: the pair's
/// children are aligned on their own, so the pairing itself carries no
/// mass of its own to prefer.
const KIND_PAIR_WEIGHT: usize = 1;

/// One endpoint resolved for alignment: the nodes its range covers and
/// every subtree beneath them by byte range, so a sibling's Merkle hash
/// and mass are one lookup.
#[derive(Debug)]
pub(super) struct Resolved<'tree> {
    /// The exact node, or the sibling window, the endpoint covers.
    nodes: Vec<&'tree NormalizedNode>,
    /// Every subtree under `nodes`, keyed by byte range.
    subtrees: HashMap<(usize, usize), Fingerprint>,
}

/// Resolves `endpoint` in its tree, or `None` when its range covers no
/// node and no sibling window.
pub(super) fn resolve<'tree>(
    tree_index: &HashMap<FileId, &'tree NormalizedNode>,
    endpoint: &Fingerprint,
) -> Option<Resolved<'tree>> {
    let root = tree_index.get(&endpoint.file_id)?;
    let nodes = resolve_range_nodes(root, endpoint.byte_range.start, endpoint.byte_range.end)?;
    let subtrees = nodes
        .iter()
        .flat_map(|node| collect_fingerprints(node, EVERY_SUBTREE))
        .map(|subtree| ((subtree.byte_range.start, subtree.byte_range.end), subtree))
        .collect();
    Some(Resolved { nodes, subtrees })
}

/// The aligned core of two resolved endpoints, in source order.
pub(super) fn aligned_core(
    left: &Resolved<'_>,
    right: &Resolved<'_>,
) -> Vec<(Fingerprint, Fingerprint)> {
    let mut core = Vec::new();
    align_siblings(&left.nodes, &right.nodes, (left, right), &mut core);
    core
}

/// Pairs two sibling sequences: Merkle-equal subtrees first, by the
/// heaviest order-preserving pairing, then, in each stretch neither side
/// matched, siblings of one kind whose children are aligned the same way.
fn align_siblings(
    left: &[&NormalizedNode],
    right: &[&NormalizedNode],
    resolved: (&Resolved<'_>, &Resolved<'_>),
    core: &mut Vec<(Fingerprint, Fingerprint)>,
) {
    let ours = digests(left, resolved.0);
    let theirs = digests(right, resolved.1);
    let matched = heaviest_subsequence(&ours, &theirs, |ours, theirs| {
        ours.hash.is_some() && ours.hash == theirs.hash
    });
    let end = (left.len(), right.len());
    let mut from = (0_usize, 0_usize);
    for (left_at, right_at) in matched.into_iter().chain(std::iter::once(end)) {
        let leftovers = (
            stretch(left, from.0, left_at),
            stretch(right, from.1, right_at),
        );
        align_leftovers(leftovers, resolved, core);
        if let Some(pair) = fingerprints(left.get(left_at), right.get(right_at), resolved) {
            core.push(pair);
        }
        from = (left_at.saturating_add(1), right_at.saturating_add(1));
    }
}

/// Pairs the siblings neither side matched by kind, descending into the
/// children of each pair; two leaves of one kind are paired outright.
///
/// Two stretches that share no kind at all may still be one code inside
/// a shell: a whole file against the namespace it declares, a class
/// against the body of the class beside it. An ordered alignment deletes
/// the shell and maps what it wrapped, so a lone container on either
/// side is unwrapped and its children aligned against the other side —
/// the left one first, the right one when the left yields nothing.
fn align_leftovers(
    leftovers: (&[&NormalizedNode], &[&NormalizedNode]),
    resolved: (&Resolved<'_>, &Resolved<'_>),
    core: &mut Vec<(Fingerprint, Fingerprint)>,
) {
    let (left, right) = leftovers;
    if left.is_empty() || right.is_empty() {
        return;
    }
    let matched = heaviest_subsequence(&kinds(left), &kinds(right), |ours, theirs| {
        ours.kind == theirs.kind
    });
    if matched.is_empty() {
        unwrap_container(left, right, resolved, core);
        return;
    }
    for (left_at, right_at) in matched {
        let (Some(ours), Some(theirs)) = (left.get(left_at), right.get(right_at)) else {
            continue;
        };
        if ours.children.is_empty() && theirs.children.is_empty() {
            core.extend(fingerprints(Some(ours), Some(theirs), resolved));
        } else {
            align_siblings(&children(ours), &children(theirs), resolved, core);
        }
    }
}

/// Aligns the children of a lone container on one side against the
/// other side's siblings: the left container first, and the right one
/// only when unwrapping the left paired nothing.
fn unwrap_container(
    left: &[&NormalizedNode],
    right: &[&NormalizedNode],
    resolved: (&Resolved<'_>, &Resolved<'_>),
    core: &mut Vec<(Fingerprint, Fingerprint)>,
) {
    let held = core.len();
    if let [container] = left {
        if !container.children.is_empty() {
            align_siblings(&children(container), right, resolved, core);
        }
    }
    if core.len() > held {
        return;
    }
    if let [container] = right {
        if !container.children.is_empty() {
            align_siblings(left, &children(container), resolved, core);
        }
    }
}

/// The subtree fingerprints of two nodes, when both are known.
fn fingerprints(
    left: Option<&&NormalizedNode>,
    right: Option<&&NormalizedNode>,
    resolved: (&Resolved<'_>, &Resolved<'_>),
) -> Option<(Fingerprint, Fingerprint)> {
    let ours = resolved.0.subtrees.get(&range_key(left?))?;
    let theirs = resolved.1.subtrees.get(&range_key(right?))?;
    Some((ours.clone(), theirs.clone()))
}

/// One sibling as the pairing sees it: its Merkle hash, `None` for a
/// node the resolution did not fingerprint — which never equals
/// anything — and the mass a pairing of it is worth.
struct Digest {
    /// The sibling's Merkle hash.
    hash: Option<[u8; 32]>,
    /// The sibling's node count.
    weight: usize,
}

/// Each sibling's digest.
fn digests(nodes: &[&NormalizedNode], resolved: &Resolved<'_>) -> Vec<Digest> {
    nodes
        .iter()
        .map(|node| {
            let found = resolved.subtrees.get(&range_key(node));
            Digest {
                hash: found.map(|subtree| subtree.hash),
                weight: found.map_or(0, |subtree| subtree.node_count),
            }
        })
        .collect()
}

/// One sibling as the leftover pairing sees it: its kind, at unit weight.
struct Kind {
    /// The sibling's normalised kind.
    kind: &'static str,
    /// The unit weight of a same-kind pairing.
    weight: usize,
}

/// Each sibling's kind.
fn kinds(nodes: &[&NormalizedNode]) -> Vec<Kind> {
    nodes
        .iter()
        .map(|node| Kind {
            kind: node.kind,
            weight: KIND_PAIR_WEIGHT,
        })
        .collect()
}

/// What a pairing of one sibling is worth.
trait Weighted {
    /// The sibling's weight.
    fn weight(&self) -> usize;
}

impl Weighted for Digest {
    fn weight(&self) -> usize {
        self.weight
    }
}

impl Weighted for Kind {
    fn weight(&self) -> usize {
        self.weight
    }
}

/// A node's children as a sibling sequence.
fn children(node: &NormalizedNode) -> Vec<&NormalizedNode> {
    node.children.iter().collect()
}

/// The lookup key of a node's byte range.
fn range_key(node: &NormalizedNode) -> (usize, usize) {
    (node.byte_range.start, node.byte_range.end)
}

/// The siblings between two positions, empty when the positions meet.
fn stretch<'seq, 'tree>(
    nodes: &'seq [&'tree NormalizedNode],
    from: usize,
    to: usize,
) -> &'seq [&'tree NormalizedNode] {
    nodes.get(from..to).unwrap_or_default()
}

/// The heaviest common subsequence of two sequences under `equal`, as
/// ascending index pairs — the order-preserving pairing an alignment
/// needs, choosing by the mass of what it pairs rather than by how many
/// pairs it makes, over sequences as short as a sibling list.
fn heaviest_subsequence<K: Weighted>(
    left: &[K],
    right: &[K],
    equal: impl Fn(&K, &K) -> bool,
) -> Vec<(usize, usize)> {
    let stride = right.len().saturating_add(1);
    let table = subsequence_table(left, right, &equal);
    let mut pairs = Vec::new();
    let (mut row, mut column) = (left.len(), right.len());
    while row > 0 && column > 0 {
        let (above, before) = (row.saturating_sub(1), column.saturating_sub(1));
        let paired = left
            .get(above)
            .zip(right.get(before))
            .is_some_and(|(ours, theirs)| {
                equal(ours, theirs)
                    && cell(&table, stride, row, column)
                        == cell(&table, stride, above, before).saturating_add(ours.weight())
            });
        if paired {
            pairs.push((above, before));
            (row, column) = (above, before);
        } else if cell(&table, stride, above, column) >= cell(&table, stride, row, before) {
            row = above;
        } else {
            column = before;
        }
    }
    pairs.reverse();
    pairs
}

/// The heaviest-subsequence table, row stride `right.len() + 1`: each
/// cell is the greatest paired mass over the prefixes it indexes.
fn subsequence_table<K: Weighted>(
    left: &[K],
    right: &[K],
    equal: &impl Fn(&K, &K) -> bool,
) -> Vec<usize> {
    let stride = right.len().saturating_add(1);
    let mut table = vec![0_usize; left.len().saturating_add(1).saturating_mul(stride)];
    for (above, ours) in left.iter().enumerate() {
        for (before, theirs) in right.iter().enumerate() {
            let (row, column) = (above.saturating_add(1), before.saturating_add(1));
            let skipped =
                cell(&table, stride, above, column).max(cell(&table, stride, row, before));
            let paired = if equal(ours, theirs) {
                cell(&table, stride, above, before).saturating_add(ours.weight())
            } else {
                0
            };
            if let Some(slot) = table.get_mut(row.saturating_mul(stride).saturating_add(column)) {
                *slot = skipped.max(paired);
            }
        }
    }
    table
}

/// One table cell, zero out of range.
fn cell(table: &[usize], stride: usize, row: usize, column: usize) -> usize {
    table
        .get(row.saturating_mul(stride).saturating_add(column))
        .copied()
        .unwrap_or_default()
}

impl<'corpus> OverlapMeasurer<'corpus> {
    /// [FUSED-SHARED-SUBTREE-CORE] The code two endpoints provably
    /// share: an order- and nesting-preserving pairing of their subtrees
    /// — Merkle-equal siblings by longest common subsequence, the
    /// leftovers by kind, descending to the leaves ([`core`]). Every
    /// pair is the same normalised code on both sides, so the content
    /// frontiers over the core align position for position and the
    /// content gate can judge a near-miss on what it shares rather than
    /// on a bag of keys diluted by the statement that made it a
    /// near-miss.
    ///
    /// The core is read from the endpoints' own resolved subtrees, never
    /// from the endpoint digests the caller carries: a Merkle-equal pair
    /// resolves to one whole-endpoint span either way, and a stale or
    /// placeholder digest cannot make two different trees one span. An
    /// unresolvable pair has no core — and no core is never evidence of
    /// anything.
    pub fn aligned_core(
        &mut self,
        left: &Fingerprint,
        right: &Fingerprint,
    ) -> Vec<(Fingerprint, Fingerprint)> {
        let resolved = self.resolved(left).zip(self.resolved(right));
        resolved.map_or_else(Vec::new, |(left, right)| aligned_core(&left, &right))
    }

    /// Returns (resolving on first use) the endpoint's subtrees for the
    /// core pairing. Retained under the same cap as the views, for the
    /// same reason: a star-shaped bucket resolves one endpoint against
    /// many partners, and a corpus holds too many distinct endpoints to
    /// keep them all.
    fn resolved(&mut self, endpoint: &Fingerprint) -> Option<Arc<Resolved<'corpus>>> {
        let key = endpoint_key(endpoint);
        if let Some(cached) = self.cores.get(&key) {
            return cached.clone();
        }
        let built = resolve(&self.tree_index, endpoint).map(Arc::new);
        if self.cores.len() < ENDPOINT_VIEW_MEMO_MAX {
            let _previous = self.cores.insert(key, built.clone());
        }
        built
    }
}
