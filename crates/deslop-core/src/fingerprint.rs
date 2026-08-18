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
    let _ = hash_and_collect(
        root,
        min_nodes,
        &mut out,
        None,
        false,
        &mut HashScratch::default(),
    );
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
    let _ = hash_and_collect(
        root,
        min_nodes,
        &mut out,
        Some(language),
        false,
        &mut HashScratch::default(),
    );
    out
}

/// Hashes `node` bottom-up, pushing a [`Fingerprint`] into `out` whenever a
/// subtree meets the minimum node count. Returns `(hash, subtree_node_count)`
/// for the caller to incorporate into its own hash.
///
/// `scratch` supplies the frame stack and hash arena; a caller invoking this
/// repeatedly over one tree ([`subtree_hash`]) reuses their capacity so the
/// walk allocates only on its first call.
fn hash_and_collect<'tree>(
    node: &'tree NormalizedNode,
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
    language: Option<&str>,
    inside_boilerplate: bool,
    scratch: &mut HashScratch<'tree>,
) -> ([u8; 32], usize) {
    let mut root_result = ([0_u8; 32], 0_usize);
    let base = scratch.hashes.len();
    scratch
        .frames
        .push(Frame::new(node, language, inside_boilerplate, base));
    while let Some(step) = next_step(&mut scratch.frames) {
        match step {
            Step::Descend(child, inherited) => {
                let base = scratch.hashes.len();
                scratch
                    .frames
                    .push(Frame::new(child, language, inherited, base));
            }
            Step::Finish => finish_top(scratch, min_nodes, out, &mut root_result),
        }
    }
    // The root's hash is returned by value; popping its arena entry restores
    // `scratch` to its caller's baseline for the next reuse.
    let _root_hash = scratch.hashes.pop();
    root_result
}

/// Pops the finished top frame, folding its result into its parent — or into
/// `root_result` when the stack empties.
fn finish_top(
    scratch: &mut HashScratch<'_>,
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
    root_result: &mut ([u8; 32], usize),
) {
    let Some(frame) = scratch.frames.pop() else {
        return;
    };
    let (hash, count) = frame.finish(min_nodes, out, &mut scratch.hashes);
    match scratch.frames.last_mut() {
        Some(parent) => parent.absorb(count),
        None => *root_result = (hash, count),
    }
}

/// Reusable buffers for [`hash_and_collect`]: the explicit frame stack and
/// the arena of finished child hashes.
///
/// [`subtree_hash`] runs once per sibling at every node of the sibling walk,
/// so a fresh allocation per call is a measured ~10% scan regression; the
/// walk instead borrows these buffers, which grow once and are then reused
/// allocation-free.
#[derive(Default)]
pub(crate) struct HashScratch<'tree> {
    /// In-progress frames, one per node on the current root-to-node path.
    frames: Vec<Frame<'tree>>,
    /// Finished child hashes; each open frame's children occupy the
    /// contiguous run starting at its [`Frame::hash_base`].
    hashes: Vec<[u8; 32]>,
}

/// One node's in-progress state on [`hash_and_collect`]'s explicit stack.
///
/// The walk is iterative rather than recursive so a file the
/// [`MAX_AST_DEPTH`] guard *accepts* cannot exhaust a 1 MB thread stack
/// (Windows' default) part-way down and abort the process with no report at
/// all — every duplicate in every other file lost. Pinned by
/// `deslop::fsharp_deep_match_stack_overflow`.
///
/// The frame is deliberately small. A `blake3::Hasher` is ~1.9 KB, and
/// holding one per open node made deep walks pay for hasher-sized memcpys on
/// every stack growth; instead children's hashes accumulate in
/// [`HashScratch::hashes`] and the hasher exists only inside
/// [`Frame::finish`], fed `kind`, the separator, then the child hashes in
/// order — byte-for-byte the digest input of the original recursive walk.
///
/// [`MAX_AST_DEPTH`]: crate::lang::shared::MAX_AST_DEPTH
struct Frame<'tree> {
    /// The node being hashed.
    node: &'tree NormalizedNode,
    /// Start of this node's finished child hashes in
    /// [`HashScratch::hashes`].
    hash_base: usize,
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
    /// Opens a frame over `node` whose children's hashes will land at
    /// `hash_base` on the shared arena.
    fn new(
        node: &'tree NormalizedNode,
        language: Option<&str>,
        inherited: bool,
        hash_base: usize,
    ) -> Self {
        Self {
            node,
            hash_base,
            next_child: 0,
            node_count: 1,
            boilerplate: inherited
                || is_boilerplate(language, node)
                || is_literal_data_subtree(node),
        }
    }

    /// Folds a finished child's node count into this frame; the child's hash
    /// is already on the arena where [`Frame::finish`] reads it.
    fn absorb(&mut self, count: usize) {
        self.node_count = self.node_count.saturating_add(count);
    }

    /// Closes the frame: digests the node over its children's arena hashes,
    /// emits a [`Fingerprint`] when the subtree qualifies, then replaces the
    /// children's arena run with this node's own hash.
    fn finish(
        self,
        min_nodes: usize,
        out: &mut Vec<Fingerprint>,
        hashes: &mut Vec<[u8; 32]>,
    ) -> ([u8; 32], usize) {
        let hash = digest_node(self.node, hashes.get(self.hash_base..).unwrap_or(&[]));
        if self.node_count >= min_nodes && !self.boilerplate && !re_describes_only_child(self.node)
        {
            out.push(Fingerprint {
                hash,
                file_id: self.node.file_id,
                byte_range: self.node.byte_range,
                node_count: self.node_count,
            });
        }
        hashes.truncate(self.hash_base);
        hashes.push(hash);
        (hash, self.node_count)
    }
}

/// Digests one node: its kind, the separator, then `child_hashes` in order
/// — byte-for-byte the digest input of the original recursive walk, which is
/// what keeps every persisted fingerprint and report hash stable.
fn digest_node(node: &NormalizedNode, child_hashes: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    let _ = hasher.update(node.kind.as_bytes());
    let _ = hasher.update(b"\0");
    for child_hash in child_hashes {
        let _ = hasher.update(child_hash);
    }
    hasher.finalize().into()
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
/// apart. Callers invoking this per sibling thread one [`HashScratch`]
/// through so repeated calls reuse the same buffers.
#[must_use]
pub(crate) fn subtree_hash<'tree>(
    node: &'tree NormalizedNode,
    scratch: &mut HashScratch<'tree>,
) -> [u8; 32] {
    let mut discarded = Vec::new();
    let (hash, _count) = hash_and_collect(node, usize::MAX, &mut discarded, None, true, scratch);
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
