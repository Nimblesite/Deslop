//! Token stream extraction from normalised AST subtrees.
//!
//! Implements the token source for [DECISION-TYPE3-TWO-PASS] / the token LSH
//! stage of [FUSED-SIGNALS-THREE-LAYER]. A "token" here is the normalised
//! `kind` of an AST node, yielded in pre-order. Identifier and literal nodes
//! have already been collapsed to `__ident__` / `__literal__` by the language
//! parser, so two Type-2 clones produce identical token streams and Type-3
//! near-misses produce streams with high k-gram Jaccard.

use crate::{
    ast::NormalizedNode,
    boilerplate::is_boilerplate,
    fingerprint::{subtree_hash, Fingerprint, HashScratch},
    sibling::window_matches_hash,
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
    let nodes = resolve_fingerprint_nodes(root, fingerprint)?;
    Some(nodes.into_iter().flat_map(token_stream).collect())
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
    for node in resolve_fingerprint_nodes(root, fingerprint)? {
        emit_node_tokens(node, &mut out, Some(language));
    }
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
    if is_boilerplate(Some(language), node) {
        return;
    }
    out.push(node.kind);
    for child in &node.children {
        walk_skipping_boilerplate(child, out, language);
    }
}

/// Resolves the exact hashed subtree or synthetic sibling window behind a
/// fingerprint. Grammar wrappers may share a byte span with their child;
/// range alone would make token, content and overlap signals inspect the
/// wrong node ([FUSED-SHARED-SUBTREE-BOUND]).
pub(crate) fn resolve_fingerprint_nodes<'tree>(
    node: &'tree NormalizedNode,
    fingerprint: &Fingerprint,
) -> Option<Vec<&'tree NormalizedNode>> {
    let range = fingerprint.byte_range;
    if node.byte_range.start > range.start || node.byte_range.end < range.end {
        return None;
    }
    if exact_fingerprint_node(node, fingerprint) {
        return Some(vec![node]);
    }
    matching_fingerprint_window(node, fingerprint).or_else(|| {
        node.children
            .iter()
            .find_map(|child| resolve_fingerprint_nodes(child, fingerprint))
    })
}

/// Checks both mass and Merkle identity before choosing an exact node.
fn exact_fingerprint_node(node: &NormalizedNode, fingerprint: &Fingerprint) -> bool {
    node.byte_range == fingerprint.byte_range
        && node.subtree_node_count() == fingerprint.node_count
        && subtree_hash(node, &mut HashScratch::default()) == fingerprint.hash
}

/// Normalisation-collapsed content frontier covered by `fingerprint`,
/// in pre-order, as `(kind, byte range)` pairs. `None` when the range
/// resolves to no node or sibling window. Feeds the content-agreement
/// signal ([FUSED-CONTENT-GATE]) — the frontier positions are exactly
/// where two shape-identical subtrees can still disagree in raw source
/// content — and the literal-dominance measurement behind the
/// language-agnostic data-table category ([CLONE-NOISE-LITERAL-TABLE]).
///
/// Import/prologue subtrees for `language` are skipped with the same
/// [`is_boilerplate`] classifier the fingerprint, sibling and token-LSH
/// passes apply ([PIPELINE-BOILERPLATE-FILTER]): those ranges are not
/// clone evidence on any other axis, so their raw bytes must neither
/// corroborate nor disprove a cluster here — one exclusion, one frontier
/// population, on every pass ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
#[must_use]
pub(crate) fn collapsed_leaves(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: Option<&str>,
) -> Option<Vec<CollapsedLeaf>> {
    let resolved = resolve_fingerprint_nodes(root, fingerprint)?;
    let mut out = Vec::new();
    let mut groups = 0_u32;
    for member in resolved {
        collect_collapsed_leaves(member, &mut out, language, &mut groups);
    }
    Some(out)
}

/// One collapsed frontier position: its normalised kind, its raw byte
/// range, and — for a fragment of a composite authored literal such as
/// an interpolated string — the literal it belongs to. Fragments of one
/// authored literal share a group so content measurement can judge the
/// literal as the human wrote it ([FUSED-CONTENT-GATE]).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CollapsedLeaf {
    /// Normalised node kind of the collapsed position.
    pub(crate) kind: &'static str,
    /// Raw source byte range of the position.
    pub(crate) range: crate::ast::ByteRange,
    /// Authored-literal group for a composite literal's fragments.
    pub(crate) literal_group: Option<u32>,
}

/// Pre-order walk collecting the content frontier as `(kind, range)`
/// pairs: a collapsed node counts only when no collapsed descendant
/// already carries its bytes. A collapsed non-leaf — a template string
/// whose fragments and interpolated identifiers are collapsed children —
/// spans those descendants, so emitting it too would re-test the same
/// bytes a second time and manufacture a disagreeing "literal" at every
/// interpolation whose identifier a Type-2 rename touched.
fn collect_collapsed_leaves(
    node: &NormalizedNode,
    out: &mut Vec<CollapsedLeaf>,
    language: Option<&str>,
    groups: &mut u32,
) {
    if is_boilerplate(language, node) {
        return;
    }
    let frontier = out.len();
    for child in &node.children {
        collect_collapsed_leaves(child, out, language, groups);
    }
    // [PIPELINE-NORMALIZE-AST-OPERATOR] An operator leaf is a frontier
    // position like any other collapsed leaf. Its kind already carries
    // the token, so members that add and subtract disagree in the
    // digest; the frontier position is what lets the content stage say
    // *which* positions disagreed rather than only that the shapes did.
    let collapsed = node.kind == crate::lang::shared::IDENTIFIER_KIND
        || node.kind == crate::lang::shared::LITERAL_KIND
        || crate::lang::shared::is_operator_kind(node.kind);
    if collapsed && out.len() == frontier {
        out.push(CollapsedLeaf {
            kind: node.kind,
            range: node.byte_range,
            literal_group: None,
        });
        return;
    }
    tag_composite_literal(node, out, frontier, groups);
}

/// Stamps the fragments a composite authored literal contributed with
/// one shared group, outermost literal winning, so a preserved fragment
/// cannot affirm a literal whose sibling fragment drifted
/// ([FUSED-CONTENT-GATE]).
fn tag_composite_literal(
    node: &NormalizedNode,
    out: &mut [CollapsedLeaf],
    frontier: usize,
    groups: &mut u32,
) {
    let Some(fragments) = out.get_mut(frontier..) else {
        return;
    };
    if node.kind != crate::lang::shared::LITERAL_KIND || fragments.is_empty() {
        return;
    }
    let group = *groups;
    *groups = groups.saturating_add(1);
    for leaf in fragments {
        if leaf.kind == crate::lang::shared::LITERAL_KIND {
            leaf.literal_group = Some(group);
        }
    }
}

/// Finds a sibling run whose count and synthetic hash match the fingerprint.
fn matching_fingerprint_window<'tree>(
    node: &'tree NormalizedNode,
    fingerprint: &Fingerprint,
) -> Option<Vec<&'tree NormalizedNode>> {
    node.children
        .iter()
        .enumerate()
        .filter(|(_, child)| child.byte_range.start == fingerprint.byte_range.start)
        .find_map(|(first_index, _)| window_from(&node.children, first_index, fingerprint))
}

/// Builds the exact matching child window, including zero-width siblings.
fn window_from<'tree>(
    children: &'tree [NormalizedNode],
    first_index: usize,
    fingerprint: &Fingerprint,
) -> Option<Vec<&'tree NormalizedNode>> {
    let mut window = Vec::new();
    for child in children.iter().skip(first_index) {
        if child.byte_range.end > fingerprint.byte_range.end {
            return None;
        }
        window.push(child);
        if child.byte_range.end == fingerprint.byte_range.end
            && window.len() > 1
            && window_mass(&window) == fingerprint.node_count
            && window_matches_hash(&window, fingerprint.hash)
        {
            return Some(window);
        }
    }
    None
}

/// Counts nodes by the same saturating rule as sibling fingerprinting.
fn window_mass(nodes: &[&NormalizedNode]) -> usize {
    nodes.iter().fold(0_usize, |sum, node| {
        sum.saturating_add(node.subtree_node_count())
    })
}

/// Emits one node with the optional language-aware boilerplate filter.
fn emit_node_tokens(node: &NormalizedNode, out: &mut Vec<&'static str>, language: Option<&str>) {
    if let Some(language) = language {
        walk_skipping_boilerplate(node, out, language);
    } else {
        walk(node, out);
    }
}
