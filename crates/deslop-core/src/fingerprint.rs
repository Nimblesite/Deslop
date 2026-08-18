//! Merkle subtree fingerprinting.
//!
//! Implements [PIPELINE-FINGERPRINT-MERKLE]: bottom-up `blake3` hash over
//! `NormalizedNode`. Each entry retains byte range, file id, subtree node
//! count, and hash so downstream clustering can group by hash and the
//! renderer can cite exact source locations.

use blake3::Hasher;

use crate::{
    ast::{ByteRange, NormalizedNode},
    boilerplate::is_boilerplate,
    lang::shared::{FILE_KIND, LITERAL_KIND},
    state::FileId,
};

/// A hashed subtree ready for clustering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// `blake3` digest of the normalised subtree.
    pub hash: [u8; 32],
    /// File this subtree lives in.
    pub file_id: FileId,
    /// Byte range spanned in the source file.
    pub byte_range: ByteRange,
    /// Total number of nodes in the subtree, including the root.
    pub node_count: usize,
}

/// Returns fingerprints for every subtree in `root` whose size is
/// `>= min_nodes`. The root itself is included when it meets the threshold.
#[must_use]
pub fn collect_fingerprints(root: &NormalizedNode, min_nodes: usize) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    let _ = hash_and_collect(root, min_nodes, &mut out, None, false);
    out
}

/// Returns fingerprints for non-boilerplate subtrees only.
#[must_use]
pub fn collect_non_boilerplate_fingerprints(
    root: &NormalizedNode,
    min_nodes: usize,
    language: &str,
) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    let _ = hash_and_collect(root, min_nodes, &mut out, Some(language), false);
    out
}

/// Hashes `node` bottom-up, pushing a [`Fingerprint`] into `out` whenever a
/// subtree meets the minimum node count. Returns `(hash, subtree_node_count)`
/// for the caller to incorporate into its own hash.
fn hash_and_collect(
    node: &NormalizedNode,
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
    language: Option<&str>,
    inside_boilerplate: bool,
) -> ([u8; 32], usize) {
    let mut stack = vec![Frame::new(node, language, inside_boilerplate)];
    let mut root_result = ([0_u8; 32], 0_usize);
    while let Some(step) = next_step(&mut stack) {
        match step {
            Step::Descend(child, inherited) => {
                stack.push(Frame::new(child, language, inherited));
            }
            Step::Finish => {
                let Some(frame) = stack.pop() else { break };
                let finished = frame.finish(min_nodes, out);
                match stack.last_mut() {
                    Some(parent) => parent.absorb(finished),
                    None => root_result = finished,
                }
            }
        }
    }
    root_result
}

/// One node's in-progress state on [`hash_and_collect`]'s explicit stack.
///
/// The walk is iterative rather than recursive because a `blake3::Hasher` is
/// ~1.9 KB and stays live for the whole of a node's visit. Recursing held one
/// per stack frame, so a file the [`MAX_AST_DEPTH`] guard *accepted* still
/// exhausted a 1 MB thread stack (Windows' default) part-way down and aborted
/// the process with no report at all — every duplicate in every other file
/// lost. Here the per-node state lives on the heap and stack depth is
/// constant. Pinned by `deslop::fsharp_deep_match_stack_overflow`.
///
/// [`MAX_AST_DEPTH`]: crate::lang::shared::MAX_AST_DEPTH
struct Frame<'tree> {
    /// The node being hashed.
    node: &'tree NormalizedNode,
    /// Running digest over the node kind and its children's hashes.
    hasher: Hasher,
    /// Index of the next child to descend into.
    next_child: usize,
    /// Nodes counted in this subtree so far, including the node itself.
    node_count: usize,
    /// Whether this node or an ancestor is boilerplate.
    boilerplate: bool,
}

/// What the walk does next with the frame on top of the stack.
enum Step<'tree> {
    /// Descend into this child, which inherits the boilerplate flag.
    Descend(&'tree NormalizedNode, bool),
    /// The top frame has no children left; the caller pops it.
    Finish,
}

impl<'tree> Frame<'tree> {
    /// Opens a frame, seeding the digest with the node kind exactly as the
    /// recursive walk did so hashes stay byte-compatible.
    fn new(node: &'tree NormalizedNode, language: Option<&str>, inherited: bool) -> Self {
        let mut hasher = Hasher::new();
        let _ = hasher.update(node.kind.as_bytes());
        let _ = hasher.update(b"\0");
        Self {
            node,
            hasher,
            next_child: 0,
            node_count: 1,
            boilerplate: inherited
                || is_boilerplate(language, node)
                || is_literal_data_subtree(node),
        }
    }

    /// Folds a finished child's hash and node count into this frame.
    fn absorb(&mut self, (hash, count): ([u8; 32], usize)) {
        let _ = self.hasher.update(&hash);
        self.node_count = self.node_count.saturating_add(count);
    }

    /// Closes the frame, emitting a [`Fingerprint`] when the subtree
    /// qualifies, and returns its hash and node count for the parent.
    fn finish(self, min_nodes: usize, out: &mut Vec<Fingerprint>) -> ([u8; 32], usize) {
        let hash: [u8; 32] = self.hasher.finalize().into();
        if self.node_count >= min_nodes
            && !self.boilerplate
            && !re_describes_only_child(self.node)
        {
            out.push(Fingerprint {
                hash,
                file_id: self.node.file_id,
                byte_range: self.node.byte_range,
                node_count: self.node_count,
            });
        }
        (hash, self.node_count)
    }
}

/// Advances the top frame: descend into its next child, or report it done.
fn next_step<'tree>(stack: &mut [Frame<'tree>]) -> Option<Step<'tree>> {
    let frame = stack.last_mut()?;
    match frame.node.children.get(frame.next_child) {
        Some(child) => {
            frame.next_child = frame.next_child.saturating_add(1);
            Some(Step::Descend(child, frame.boilerplate))
        }
        None => Some(Step::Finish),
    }
}

/// The bottom-up merkle hash of `node`, matching [`collect_fingerprints`].
///
/// Iterative for the same reason as [`hash_and_collect`], whose scheme it
/// shares — one hash definition, so a change to either cannot drift them
/// apart.
#[must_use]
pub(crate) fn subtree_hash(node: &NormalizedNode) -> [u8; 32] {
    let mut discarded = Vec::new();
    let (hash, _count) = hash_and_collect(node, usize::MAX, &mut discarded, None, true);
    hash
}

/// True when the synthetic `__file__` root adds nothing to its only child.
///
/// [PIPELINE-NORMALIZE-AST] gives the root the extent of the nodes
/// normalisation kept, so a file holding a single declaration yields a root
/// whose byte range — and therefore whose source text — is identical to that
/// declaration's. Fingerprinting both reports one region twice: it
/// double-counts in `clusters_total` and the duplication metric, and because
/// the two spans carry byte-identical text the embedding pass scores them a
/// perfect match *inside a single file*, seeding clusters through transitive
/// closure that describe no duplication at all.
///
/// Only the synthetic root is suppressed, and only when a single child covers
/// it exactly. That child is always fingerprinted in its place, and any
/// cluster the root could have joined the child joins on the same bytes, so
/// no finding is lost. Pinned by `deslop::issue_343_sum_clamp_saturation`.
fn re_describes_only_child(node: &NormalizedNode) -> bool {
    node.kind == FILE_KIND
        && matches!(node.children.as_slice(), [only] if only.byte_range == node.byte_range)
}

/// Half-open overlap test on two fingerprints' byte ranges.
pub(crate) fn ranges_overlap(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.byte_range.start < right.byte_range.end && right.byte_range.start < left.byte_range.end
}

/// Returns true for one literal-data element inside a literal-only block.
pub(crate) fn is_literal_data_item(node: &NormalizedNode) -> bool {
    node.kind == LITERAL_KIND || is_literal_data_subtree(node)
}

/// Returns true for Python-style literal-only data containers.
pub(crate) fn is_literal_data_subtree(node: &NormalizedNode) -> bool {
    is_literal_data_carrier(node.kind)
        && !node.children.is_empty()
        && node.children.iter().all(is_literal_data_item)
}

/// Returns true for normalized node kinds that only arrange literal data.
fn is_literal_data_carrier(kind: &str) -> bool {
    matches!(kind, "dictionary" | "pair" | "list" | "tuple" | "set")
}
