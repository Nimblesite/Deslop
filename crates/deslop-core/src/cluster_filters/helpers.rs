//! Snippet, range and tree helpers shared by every cluster filter.
//!
//! Each filter re-parses its members' real source and asks the same small
//! questions — does this node overlap the reported range, do the members
//! span two files, is every member one language — so the answers live
//! here once and the per-language modules stay about their own shapes.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::snippets::Snippet;
use crate::{
    ast::{named_children, named_children_touching, ByteRange},
    state::FileId,
};

/// Trims surrounding ASCII whitespace from a reported snippet range so
/// parser lookups are not defeated by trailing newlines outside the AST
/// node that produced the fingerprint.
pub(super) fn trimmed_snippet_range(snippet: &Snippet<'_>) -> Option<ByteRange> {
    let bytes = snippet_range_text(snippet)?;
    let leading = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let trailing = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace())?;
    let start = snippet.range.start.checked_add(leading)?;
    let end = snippet.range.start.checked_add(trailing)?.checked_add(1)?;
    Some(ByteRange { start, end })
}

/// Returns true when `node` overlaps `range`.
pub(super) fn node_intersects_range(node: Node<'_>, range: ByteRange) -> bool {
    node.start_byte() < range.end && node.end_byte() > range.start
}

/// Returns true when `file_ids` cover at least two distinct files.
///
/// Cluster-noise filters require cross-file spread before they fire: a
/// pattern repeated only within a single file is not the cross-file
/// duplication they guard against. Centralising the `BTreeSet` count keeps
/// every per-language filter's "≥ 2 distinct files" gate identical.
pub(super) fn spans_multiple_files(file_ids: impl IntoIterator<Item = FileId>) -> bool {
    file_ids.into_iter().collect::<BTreeSet<FileId>>().len() >= 2
}

/// Returns true when `snippets` is a multi-member cluster whose every member
/// is written in `language`. This is the precondition every language-specific
/// cluster filter shares before it inspects member shapes — centralising it
/// keeps the "≥ 2 members, all one language" gate identical across filters.
pub(super) fn is_multi_member_language_cluster(snippets: &[Snippet<'_>], language: &str) -> bool {
    snippets.len() >= 2 && snippets.iter().all(|snippet| snippet.language == language)
}

/// Returns every member's shape when `snippets` is a multi-member cluster
/// written wholly in `language` and `shape_of` recognises every member;
/// `None` otherwise. Every shape-comparing language filter opens this
/// way, so the gate and the all-or-nothing collection live here once.
pub(super) fn language_cluster_shapes<'snip, 'src, Shape>(
    snippets: &'snip [Snippet<'src>],
    language: &str,
    shape_of: impl FnMut(&'snip Snippet<'src>) -> Option<Shape>,
) -> Option<Vec<Shape>> {
    is_multi_member_language_cluster(snippets, language)
        .then(|| snippets.iter().map(shape_of).collect())
        .flatten()
}

/// Returns true when at least two raw reported snippet ranges differ.
pub(super) fn raw_snippet_texts_differ(snippets: &[Snippet<'_>]) -> bool {
    let Some(first) = snippets.first().and_then(snippet_range_text) else {
        return false;
    };
    snippets
        .iter()
        .filter_map(snippet_range_text)
        .any(|text| text != first)
}

/// Walks `node` looking for a named descendant of `kind`.
pub(super) fn node_contains_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| node_contains_kind(child, kind))
}

/// Returns true when `needle` occurs in `bytes`.
pub(super) fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
}

/// Returns the bytes covered by one cluster occurrence.
pub(super) fn snippet_range_text<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    snippet.source.get(snippet.range.start..snippet.range.end)
}

/// Keeps generated-file detection focused on file headers.
pub(super) fn source_head(source: &[u8]) -> &[u8] {
    let end = source.len().min(1024);
    source.get(..end).unwrap_or(source)
}

/// Trims ASCII whitespace without allocating.
pub(super) fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes.get(first..).unwrap_or_default()
}

/// Walks `node` looking for an identifier with the requested bytes.
pub(super) fn node_contains_identifier(node: Node<'_>, source: &[u8], needle: &[u8]) -> bool {
    if node.kind() == "identifier" && source.get(node.start_byte()..node.end_byte()) == Some(needle)
    {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| node_contains_identifier(child, source, needle))
}

/// Walks `root` looking for the smallest descendant of `kinds` whose
/// byte range encloses `range`.
pub(crate) fn enclosing_kind<'tree>(
    root: Node<'tree>,
    range: ByteRange,
    kinds: &[&str],
) -> Option<Node<'tree>> {
    let mut best: Option<Node<'tree>> = None;
    let mut stack: Vec<Node<'tree>> = vec![root];
    while let Some(node) = stack.pop() {
        if node.start_byte() > range.start || node.end_byte() < range.end {
            continue;
        }
        if kinds.contains(&node.kind()) {
            best = Some(node);
        }
        stack.extend(named_children_touching(node, range));
    }
    best
}
