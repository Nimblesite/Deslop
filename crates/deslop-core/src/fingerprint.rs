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
    lang::shared::LITERAL_KIND,
    state::FileId,
};

/// A hashed subtree ready for clustering.
#[derive(Debug, Clone)]
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
    let mut hasher = Hasher::new();
    let _ = hasher.update(node.kind.as_bytes());
    let _ = hasher.update(b"\0");
    let mut subtree_node_count: usize = 1;
    let current_boilerplate =
        inside_boilerplate || is_boilerplate(language, node) || is_literal_data_subtree(node);
    for child in &node.children {
        let (child_hash, child_size) =
            hash_and_collect(child, min_nodes, out, language, current_boilerplate);
        let _ = hasher.update(&child_hash);
        subtree_node_count = subtree_node_count.saturating_add(child_size);
    }
    let hash: [u8; 32] = hasher.finalize().into();
    if subtree_node_count >= min_nodes && !current_boilerplate {
        out.push(Fingerprint {
            hash,
            file_id: node.file_id,
            byte_range: node.byte_range,
            node_count: subtree_node_count,
        });
    }
    (hash, subtree_node_count)
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
