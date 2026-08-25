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
    boilerplate::{is_boilerplate, is_import_boilerplate_only_subtree},
    fingerprint::{
        is_literal_data_item, is_literal_data_subtree, subtree_hash, Fingerprint, HashScratch,
    },
};

/// Synthetic node kind used as the hash prefix for a sibling window. The
/// prefix is length-aware so a window of 3 siblings does not collide with
/// a window of 4, even if the first 3 children are identical.
const SIBLING_WINDOW_KIND: &str = "__sibling_window__";

/// Maximum sibling-window width. Caps quadratic growth at parents with many
/// children — the typical Type-3 near-miss clone spans a handful of
/// statements, not an entire class body. Values above 8 in practice only
/// rediscover matches the exact subtree pass already emitted.
pub const MAX_WINDOW_WIDTH: usize = 8;

/// Emits a [`Fingerprint`] for every contiguous sibling window whose total
/// subtree node count meets `min_nodes`. Singleton windows are skipped —
/// those are already covered by [`collect_fingerprints`] on the subtree
/// itself. The root node contributes its own children windows recursively.
#[must_use]
pub fn collect_sibling_fingerprints(root: &NormalizedNode, min_nodes: usize) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    walk(
        root,
        min_nodes,
        &mut out,
        None,
        false,
        &mut HashScratch::default(),
    );
    out
}

/// Emits sibling-window fingerprints excluding import/prologue boilerplate.
#[must_use]
pub fn collect_non_boilerplate_sibling_fingerprints(
    root: &NormalizedNode,
    min_nodes: usize,
    language: &str,
) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    walk(
        root,
        min_nodes,
        &mut out,
        Some(language),
        false,
        &mut HashScratch::default(),
    );
    out
}

/// Recursively inspects `node`'s children, emitting sibling-window
/// fingerprints whose aggregated node count clears `min_nodes`. One
/// `scratch` is threaded through the whole walk so every
/// [`subtree_hash`] call reuses the same buffers instead of allocating.
fn walk<'tree>(
    node: &'tree NormalizedNode,
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
    language: Option<&str>,
    inside_boilerplate: bool,
    scratch: &mut HashScratch<'tree>,
) {
    let current_boilerplate =
        inside_boilerplate || is_boilerplate(language, node) || is_literal_data_subtree(node);
    if !current_boilerplate {
        emit_windows(&node.children, min_nodes, out, language, scratch);
    }
    for child in &node.children {
        walk(
            child,
            min_nodes,
            out,
            language,
            current_boilerplate,
            scratch,
        );
    }
}

/// Returns `true` when every element of `hashes` is identical.
///
/// A uniform sibling sequence means every same-width window is trivially
/// hash-equivalent — none of those windows represent real duplicates since
/// the clusterer would see thousands of identical fingerprints. Individual
/// subtree fingerprinting already covers the case where multiple identical
/// subtrees exist side-by-side.
fn all_hashes_uniform(hashes: &[[u8; 32]]) -> bool {
    match hashes.first() {
        None => true,
        Some(first) => hashes.iter().all(|h| h == first),
    }
}

/// Scans every contiguous sibling window of length ≥2 in `siblings`,
/// pushing one fingerprint per window that clears `min_nodes`. Uses
/// `slice::windows` for every window width from 2 up to
/// [`MAX_WINDOW_WIDTH`] so each enumerated slice is guaranteed
/// non-empty — removes the "window can be empty" branch that was
/// previously impossible to exercise from a test.
///
/// [PIPELINE-FINGERPRINT-MERKLE] BUG #61 is handled in [`walk`] via
/// [`is_literal_data_subtree`]: literal-only containers (dicts, lists) are
/// treated as boilerplate before `emit_windows` is ever reached, so
/// their child entries never enter the sibling window fingerprinter.
fn emit_windows<'tree>(
    siblings: &'tree [NormalizedNode],
    min_nodes: usize,
    out: &mut Vec<Fingerprint>,
    language: Option<&str>,
    scratch: &mut HashScratch<'tree>,
) {
    let cumulative = cumulative_node_counts(siblings);
    let child_hashes: Vec<[u8; 32]> = siblings
        .iter()
        .map(|sibling| subtree_hash(sibling, scratch))
        .collect();
    // [PIPELINE-FINGERPRINT-MERKLE] BUG #61: when every sibling hashes
    // identically after normalisation (e.g. a C# repetitive pattern), every
    // same-width window is trivially equal — not a real clone. Individual
    // subtree fingerprinting already captures those. Skip all windows to
    // avoid an O(n²) explosion of redundant clusters.
    if all_hashes_uniform(&child_hashes) {
        return;
    }
    for width in 2..=MAX_WINDOW_WIDTH {
        for (start, window) in siblings.windows(width).enumerate() {
            if boilerplate_window(language, window) {
                continue;
            }
            if literal_data_window(window) {
                continue;
            }
            let end = start.saturating_add(width);
            let node_count = window_node_count(&cumulative, start, end);
            if node_count < min_nodes {
                continue;
            }
            // `slice::windows(w)` with `w >= 2` always yields slices
            // with a first and a last; the slice pattern makes that
            // guarantee visible to the compiler.
            if let [first, .., last] = window {
                let hash = hash_window(&child_hashes, start, end);
                out.push(window_fingerprint(first, last, hash, node_count));
            }
        }
    }
}

/// Returns true when `window` contains import/prologue boilerplate.
fn boilerplate_window(language: Option<&str>, window: &[NormalizedNode]) -> bool {
    language.is_some_and(|lang| {
        window
            .iter()
            .any(|node| is_import_boilerplate_only_subtree(lang, node))
    })
}

/// Returns true when every sibling in `window` is literal data.
fn literal_data_window(window: &[NormalizedNode]) -> bool {
    window.iter().all(is_literal_data_item)
}

/// Materialises one sibling-window fingerprint covering
/// `(first, last)`. Callers pass the endpoints explicitly — both come
/// from `slice::first` / `slice::last` on a `slice::windows(w)` slice
/// where `w >= 2`, so they are always `Some` by construction and the
/// `Option` is unwrapped at the call site.
fn window_fingerprint(
    first: &NormalizedNode,
    last: &NormalizedNode,
    hash: [u8; 32],
    node_count: usize,
) -> Fingerprint {
    Fingerprint {
        hash,
        file_id: first.file_id,
        byte_range: ByteRange {
            start: first.byte_range.start,
            end: last.byte_range.end,
        },
        node_count,
    }
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
