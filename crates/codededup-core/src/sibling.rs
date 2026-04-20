//! Sibling-sequence fingerprinting ([DECISION-TYPE3-TWO-PASS], step 1).
//!
//! Chilowicz et al. 2009 ([TECH-AST-FINGERPRINT]) extend exact subtree
//! clones to near-miss clones by treating **contiguous sibling sequences**
//! under a shared parent as first-class fingerprint inputs. This module
//! emits one [`Fingerprint`] per contiguous sibling window whose combined
//! node count is ≥ `min_nodes`. The existing hash-bucket clusterer then
//! groups matching sibling windows the same way it groups matching
//! subtrees — no new clustering code required.
//!
//! Byte ranges on sibling fingerprints span from the first sibling's start
//! to the last sibling's end, so the renderer still produces exact byte
//! ranges for Type-3 candidates discovered this way.

use blake3::Hasher;

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
};

/// Synthetic node kind used as the hash prefix for a sibling window. The
/// prefix is length-aware so a window of 3 siblings does not collide with
/// a window of 4, even if the first 3 children are identical.
const SIBLING_WINDOW_KIND: &str = "__sibling_window__";

/// Maximum sibling-window width. Caps quadratic growth at parents with many
/// children — the typical Type-3 near-miss clone spans a handful of
/// statements, not an entire class body. Values above 8 in practice only
/// rediscover matches the exact subtree pass already emitted.
const MAX_WINDOW_WIDTH: usize = 8;

/// Emits a [`Fingerprint`] for every contiguous sibling window whose total
/// subtree node count meets `min_nodes`. Singleton windows are skipped —
/// those are already covered by [`collect_fingerprints`] on the subtree
/// itself. The root node contributes its own children windows recursively.
#[must_use]
pub fn collect_sibling_fingerprints(root: &NormalizedNode, min_nodes: usize) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    walk(root, min_nodes, &mut out);
    out
}

/// Recursively inspects `node`'s children, emitting sibling-window
/// fingerprints whose aggregated node count clears `min_nodes`.
fn walk(node: &NormalizedNode, min_nodes: usize, out: &mut Vec<Fingerprint>) {
    emit_windows(&node.children, min_nodes, out);
    for child in &node.children {
        walk(child, min_nodes, out);
    }
}

/// Scans every contiguous sibling window of length ≥2 in `siblings`,
/// pushing one fingerprint per window that clears `min_nodes`.
fn emit_windows(siblings: &[NormalizedNode], min_nodes: usize, out: &mut Vec<Fingerprint>) {
    let cumulative = cumulative_node_counts(siblings);
    let child_hashes: Vec<[u8; 32]> = siblings.iter().map(subtree_hash).collect();
    for start in 0..siblings.len() {
        let max_end = start.saturating_add(MAX_WINDOW_WIDTH).min(siblings.len());
        for end in start.saturating_add(2)..=max_end {
            let node_count = window_node_count(&cumulative, start, end);
            if node_count < min_nodes {
                continue;
            }
            out.push(window_fingerprint(
                siblings,
                &child_hashes,
                start,
                end,
                node_count,
            ));
        }
    }
}

/// Materialises one sibling-window fingerprint covering `siblings[start..end]`.
fn window_fingerprint(
    siblings: &[NormalizedNode],
    child_hashes: &[[u8; 32]],
    start: usize,
    end: usize,
    node_count: usize,
) -> Fingerprint {
    let hash = hash_window(child_hashes, start, end);
    let first = siblings.get(start);
    let last_index = end.saturating_sub(1);
    let last = siblings.get(last_index);
    let file_id = first.map_or_else(
        || last.map_or_else(default_file_id, |node| node.file_id),
        |node| node.file_id,
    );
    let byte_range = ByteRange {
        start: first.map_or(0, |node| node.byte_range.start),
        end: last.map_or(0, |node| node.byte_range.end),
    };
    Fingerprint {
        hash,
        file_id,
        byte_range,
        node_count,
    }
}

/// Returns `FileId(0)` for windows whose siblings slice is empty. The empty
/// case is impossible in practice because `emit_windows` guards `end >=
/// start + 2`, but the compiler can't see that — this keeps `unwrap`/`expect`
/// out of the production path.
fn default_file_id() -> crate::state::FileId {
    crate::state::FileRegistry::new().register(std::path::PathBuf::new())
}

/// Builds a prefix-sum table of subtree node counts so window totals are
/// `O(1)` per window rather than `O(window_width)`.
fn cumulative_node_counts(siblings: &[NormalizedNode]) -> Vec<usize> {
    let mut cumulative = Vec::with_capacity(siblings.len().saturating_add(1));
    cumulative.push(0_usize);
    let mut running: usize = 0;
    for sibling in siblings {
        running = running.saturating_add(sibling.subtree_node_count());
        cumulative.push(running);
    }
    cumulative
}

/// Reads a window sum out of the prefix-sum table.
fn window_node_count(cumulative: &[usize], start: usize, end: usize) -> usize {
    let end_value = cumulative.get(end).copied().unwrap_or(0);
    let start_value = cumulative.get(start).copied().unwrap_or(0);
    end_value.saturating_sub(start_value)
}

/// Hashes the children in `child_hashes[start..end]` prefixed by
/// [`SIBLING_WINDOW_KIND`] and the window length, so windows of different
/// widths never collide.
fn hash_window(child_hashes: &[[u8; 32]], start: usize, end: usize) -> [u8; 32] {
    let mut hasher = Hasher::new();
    let _ = hasher.update(SIBLING_WINDOW_KIND.as_bytes());
    let _ = hasher.update(b"\0");
    let width = u32::try_from(end.saturating_sub(start)).unwrap_or(u32::MAX);
    let _ = hasher.update(&width.to_le_bytes());
    for index in start..end {
        let child_hash = child_hashes.get(index).copied().unwrap_or([0_u8; 32]);
        let _ = hasher.update(&child_hash);
    }
    hasher.finalize().into()
}

/// Re-hashes a subtree using the same bottom-up scheme as
/// [`crate::fingerprint`]. Kept local to avoid threading more state through
/// the pipeline; the cost is one additional pass which is `O(n)` in node
/// count.
fn subtree_hash(node: &NormalizedNode) -> [u8; 32] {
    let mut hasher = Hasher::new();
    let _ = hasher.update(node.kind.as_bytes());
    let _ = hasher.update(b"\0");
    for child in &node.children {
        let child_hash = subtree_hash(child);
        let _ = hasher.update(&child_hash);
    }
    hasher.finalize().into()
}
