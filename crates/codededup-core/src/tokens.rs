//! Token stream extraction from normalised AST subtrees.
//!
//! Implements the token source for [DECISION-TYPE3-TWO-PASS] / the token LSH
//! stage of [FUSION-SIGNALS-THREE-LAYER]. A "token" here is the normalised
//! `kind` of an AST node, yielded in pre-order. Identifier and literal nodes
//! have already been collapsed to `__ident__` / `__literal__` by the language
//! parser, so two Type-2 clones produce identical token streams and Type-3
//! near-misses produce streams with high k-gram Jaccard.

use crate::{ast::NormalizedNode, fingerprint::Fingerprint};

/// k-gram width used by the token LSH pass. Matches the value recommended by
/// the [TECH-TOKEN-SOURCERERCC] literature: short enough to keep Jaccard
/// sensitive to small edits, long enough to suppress noise from trivia.
pub const KGRAM_WIDTH: usize = 5;

/// Returns the pre-order token stream of `root`. Each token is the
/// (already normalised) node `kind`, so the output is stable across runs and
/// cheap to hash.
#[must_use]
pub fn token_stream(root: &NormalizedNode) -> Vec<&'static str> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// Extracts the token stream for a specific subtree inside `root`, located by
/// its byte range. Used by the token LSH pass so that each fingerprint
/// corresponds to a known (file, subtree) occurrence.
#[must_use]
pub fn token_stream_for_fingerprint<'a>(
    root: &'a NormalizedNode,
    fingerprint: &Fingerprint,
) -> Option<Vec<&'static str>> {
    let node = locate(root, fingerprint.byte_range.start, fingerprint.byte_range.end)?;
    Some(token_stream(node))
}

/// Computes the set of contiguous k-grams from `tokens`. Returns an empty
/// vector when `tokens.len() < k` — callers treat that as "no similarity
/// signal from this subtree."
#[must_use]
pub fn kgrams<'a>(tokens: &'a [&'static str], k: usize) -> Vec<&'a [&'static str]> {
    if k == 0 || tokens.len() < k {
        return Vec::new();
    }
    let last_start = tokens.len().saturating_sub(k);
    (0..=last_start).map(|start| window(tokens, start, k)).collect()
}

/// Returns the k-wide slice of `tokens` starting at `start`. Split into its
/// own helper so the `kgrams` loop stays within the 20-line function budget
/// without `#[allow(clippy::indexing_slicing)]`.
fn window<'a>(tokens: &'a [&'static str], start: usize, k: usize) -> &'a [&'static str] {
    let end = start.saturating_add(k).min(tokens.len());
    tokens.get(start..end).unwrap_or(&[])
}

/// Recursively emits node kinds in pre-order.
fn walk(node: &NormalizedNode, out: &mut Vec<&'static str>) {
    out.push(node.kind);
    for child in &node.children {
        walk(child, out);
    }
}

/// Returns the subtree of `node` whose byte range exactly matches
/// `[start, end)`, searching depth-first. Returns `None` when no such subtree
/// exists (e.g. because the fingerprint belongs to a different file).
fn locate(node: &NormalizedNode, start: usize, end: usize) -> Option<&NormalizedNode> {
    if node.byte_range.start == start && node.byte_range.end == end {
        return Some(node);
    }
    if node.byte_range.start > start || node.byte_range.end < end {
        return None;
    }
    for child in &node.children {
        if let Some(found) = locate(child, start, end) {
            return Some(found);
        }
    }
    None
}
