//! Token stream extraction from normalised AST subtrees.
//!
//! Implements the token source for [DECISION-TYPE3-TWO-PASS] / the token LSH
//! stage of [FUSION-SIGNALS-THREE-LAYER]. A "token" here is the normalised
//! `kind` of an AST node, yielded in pre-order. Identifier and literal nodes
//! have already been collapsed to `__ident__` / `__literal__` by the language
//! parser, so two Type-2 clones produce identical token streams and Type-3
//! near-misses produce streams with high k-gram Jaccard.

use crate::{
    ast::NormalizedNode,
    boilerplate::{is_import_boilerplate_carrier, is_import_boilerplate_only_subtree},
    fingerprint::Fingerprint,
};

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
pub fn token_stream_for_fingerprint(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
) -> Option<Vec<&'static str>> {
    let node = locate(
        root,
        fingerprint.byte_range.start,
        fingerprint.byte_range.end,
    )?;
    Some(token_stream(node))
}

/// Like [`token_stream_for_fingerprint`], but skips kinds that are
/// import/prologue boilerplate for `language` (imports, decorators,
/// namespace carriers) and can resolve synthetic sibling-window byte
/// ranges. The `MinHash` signature built from this stream therefore
/// reflects non-boilerplate code only — two files with similar import
/// prologues no longer collide into false-positive LSH-only clusters
/// ([PIPELINE-BOILERPLATE-FILTER]).
#[must_use]
pub fn token_stream_for_fingerprint_with_language(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: &str,
) -> Option<Vec<&'static str>> {
    let mut out = Vec::new();
    collect_tokens_in_range(
        root,
        fingerprint.byte_range.start,
        fingerprint.byte_range.end,
        &mut out,
        Some(language),
    )?;
    Some(out)
}

/// Like [`token_stream_for_fingerprint_with_language`], but folds common
/// control-flow and declaration node kinds into cross-language aliases.
/// Used only when the user explicitly opts into cross-language audits.
#[must_use]
pub fn cross_language_token_stream_for_fingerprint(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: &str,
) -> Option<Vec<&'static str>> {
    token_stream_for_fingerprint_with_language(root, fingerprint, language).map(|tokens| {
        tokens
            .into_iter()
            .map(cross_language_token)
            .collect::<Vec<_>>()
    })
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
    (0..=last_start)
        .map(|start| window(tokens, start, k))
        .collect()
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

/// Maps language-specific grammar names onto shared audit tokens.
fn cross_language_token(token: &'static str) -> &'static str {
    match token {
        "method_declaration" | "function_item" | "function_definition" => "__fn__",
        "parameter_list" | "parameters" => "__params__",
        "if_statement" | "if_expression" => "__if__",
        "for_statement" | "for_expression" => "__for__",
        "return_statement" | "return_expression" => "__return__",
        "binary_expression" | "binary_operator" | "comparison_operator" => "__binary__",
        "assignment_expression"
        | "assignment"
        | "let_declaration"
        | "local_declaration_statement"
        | "variable_declaration"
        | "variable_declarator" => "__assign__",
        "call" | "invocation_expression" => "__call__",
        "argument_list" => "__args__",
        "range_expression" => "__range__",
        "expression_statement" => "__expr__",
        "modifier" | "visibility_modifier" | "mutable_specifier" => "__modifier__",
        other => other,
    }
}

/// Pre-order walker that drops import/prologue subtrees for `language`.
/// Mirrors the filter applied in [`crate::fingerprint`] and
/// [`crate::sibling`] so the token LSH path sees the same code the
/// structural pass considered meaningful.
fn walk_skipping_boilerplate(node: &NormalizedNode, out: &mut Vec<&'static str>, language: &str) {
    if is_import_boilerplate_carrier(language, node.kind)
        || is_import_boilerplate_only_subtree(language, node)
    {
        return;
    }
    out.push(node.kind);
    for child in &node.children {
        walk_skipping_boilerplate(child, out, language);
    }
}

/// Returns the subtree of `node` whose byte range exactly matches
/// `[start, end)`, searching depth-first. Returns `None` when no such subtree
/// exists (e.g. because the fingerprint belongs to a synthetic sibling range).
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

/// Emits tokens for an exact node or synthetic sibling window range.
fn collect_tokens_in_range(
    node: &NormalizedNode,
    start: usize,
    end: usize,
    out: &mut Vec<&'static str>,
    language: Option<&str>,
) -> Option<()> {
    let resolved = resolve_range_nodes(node, start, end)?;
    for member in resolved {
        emit_node_tokens(member, out, language);
    }
    Some(())
}

/// Resolves a fingerprint byte range to the nodes it spans: the exact
/// node when one exists, else the contiguous child window a synthetic
/// sibling range covers. Shared by the token stream extraction and the
/// content-agreement walk ([FUSION-CONTENT-GATE]) so the two signals
/// always see the same code.
fn resolve_range_nodes(
    node: &NormalizedNode,
    start: usize,
    end: usize,
) -> Option<Vec<&NormalizedNode>> {
    if node.byte_range.start == start && node.byte_range.end == end {
        return Some(vec![node]);
    }
    if node.byte_range.start > start || node.byte_range.end < end {
        return None;
    }
    if let Some(window) = matching_child_window(node, start, end) {
        return Some(window);
    }
    node.children
        .iter()
        .find_map(|child| resolve_range_nodes(child, start, end))
}

/// Normalisation-collapsed leaves (identifier and literal nodes) covered
/// by `fingerprint`, in pre-order, as `(kind, byte range)` pairs. `None`
/// when the range resolves to no node or sibling window. Feeds the
/// content-agreement signal ([FUSION-CONTENT-GATE]) — the
/// collapsed leaves are exactly the positions where two shape-identical
/// subtrees can still disagree in raw source content — and the
/// literal-dominance measurement behind the language-agnostic data-table
/// category ([CLONE-NOISE-LITERAL-TABLE]).
#[must_use]
pub(crate) fn collapsed_leaves(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
) -> Option<Vec<(&'static str, crate::ast::ByteRange)>> {
    let resolved = resolve_range_nodes(
        root,
        fingerprint.byte_range.start,
        fingerprint.byte_range.end,
    )?;
    let mut out = Vec::new();
    for member in resolved {
        collect_collapsed_leaves(member, &mut out);
    }
    Some(out)
}

/// Pre-order walk collecting collapsed leaves as `(kind, range)` pairs.
fn collect_collapsed_leaves(
    node: &NormalizedNode,
    out: &mut Vec<(&'static str, crate::ast::ByteRange)>,
) {
    if node.kind == crate::lang::shared::IDENTIFIER_KIND
        || node.kind == crate::lang::shared::LITERAL_KIND
    {
        out.push((node.kind, node.byte_range));
    }
    for child in &node.children {
        collect_collapsed_leaves(child, out);
    }
}

/// Returns the contiguous children spanned by `[start, end)`.
fn matching_child_window(
    node: &NormalizedNode,
    start: usize,
    end: usize,
) -> Option<Vec<&NormalizedNode>> {
    for (first_index, child) in node.children.iter().enumerate() {
        if child.byte_range.start == start {
            return window_from(&node.children, first_index, end);
        }
    }
    None
}

/// Builds a child window starting at `first_index` when it ends at `end`.
fn window_from(
    children: &[NormalizedNode],
    first_index: usize,
    end: usize,
) -> Option<Vec<&NormalizedNode>> {
    let mut window = Vec::new();
    for child in children.iter().skip(first_index) {
        if child.byte_range.end > end {
            return None;
        }
        window.push(child);
        if child.byte_range.end == end {
            return Some(window);
        }
    }
    None
}

/// Emits one node with the optional language-aware boilerplate filter.
fn emit_node_tokens(node: &NormalizedNode, out: &mut Vec<&'static str>, language: Option<&str>) {
    if let Some(language) = language {
        walk_skipping_boilerplate(node, out, language);
    } else {
        walk(node, out);
    }
}
