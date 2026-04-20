//! Merkle subtree fingerprinting.
//!
//! Implements [PIPELINE-FINGERPRINT-MERKLE]: bottom-up `blake3` hash over
//! `NormalizedNode`. Each entry retains byte range, file id, subtree node
//! count, and hash so downstream clustering can group by hash and the
//! renderer can cite exact source locations.

use blake3::Hasher;

use crate::{
    ast::{ByteRange, NormalizedNode},
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
    let _ = hash_and_collect(root, min_nodes, &mut out);
    out
}

/// Hashes `node` bottom-up, pushing a [`Fingerprint`] into `out` whenever a
/// subtree meets the minimum node count. Returns `(hash, subtree_node_count)`
/// for the caller to incorporate into its own hash.
fn hash_and_collect(
    node: &NormalizedNode,
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
) -> ([u8; 32], usize) {
    let mut hasher = Hasher::new();
    let _ = hasher.update(node.kind.as_bytes());
    let _ = hasher.update(b"\0");
    let mut subtree_node_count: usize = 1;
    for child in &node.children {
        let (child_hash, child_size) = hash_and_collect(child, min_nodes, out);
        let _ = hasher.update(&child_hash);
        subtree_node_count = subtree_node_count.saturating_add(child_size);
    }
    let hash: [u8; 32] = hasher.finalize().into();
    if subtree_node_count >= min_nodes {
        out.push(Fingerprint {
            hash,
            file_id: node.file_id,
            byte_range: node.byte_range,
            node_count: subtree_node_count,
        });
    }
    (hash, subtree_node_count)
}
