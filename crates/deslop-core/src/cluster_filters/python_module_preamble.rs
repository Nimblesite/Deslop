//! Module-preamble sibling-window filter for Python test files.
//!
//! Issue [CLONE-NOISE-PY-MODULE-PREAMBLE]: the sibling-window
//! fingerprinter emits a fingerprint over a contiguous run of >=2
//! module-level definitions. A test module that opens with several small
//! helpers/fixtures therefore matches any other test module whose
//! preamble has the same number of equally-shaped definitions, reaching
//! `structural=1.00, token_jaccard=1.00` after Type-2 normalisation. The
//! match unit is a "block of declarations", not a coherent code unit —
//! the embedding pass correctly scores it ~0.
//!
//! We suppress such a cluster **only** when no two members share
//! identical definition bodies. A helper module copied verbatim across
//! files (or copy-pasted then renamed, since renaming leaves the body
//! bytes intact) has body-equivalent members and is kept — that is real,
//! extractable duplication. Keying on body divergence rather than name
//! divergence is what distinguishes a coincidental preamble shape from a
//! genuine copy, mirroring [`super::enclosing_function_bodies_differ`].

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    language_cluster_shapes, parse_for, spans_multiple_files, trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

/// Top-level definition kinds that make up a Python test-module preamble.
/// `class_definition` is intentionally excluded: class-shape false
/// positives are handled by [`super::python_class_shapes`], and counting
/// classes here would widen the detector beyond the function-preamble
/// target (e.g. distinct ORM model classes).
const PREAMBLE_KINDS: &[&str] = &["function_definition", "decorated_definition"];

/// Returns true when every member's matched range spans a run of >=2
/// sibling top-level definitions and no two members share identical
/// definition bodies — the coincidental-preamble-shape signature of
/// issue. Returns false (keeps the cluster) when any member is
/// not such a multi-definition run, or when two members are body
/// equivalent (a genuine copy that must still surface).
pub(super) fn is_module_preamble_sequence_cluster(snippets: &[Snippet<'_>]) -> bool {
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && language_cluster_shapes(snippets, "python", member_preamble_bodies)
            .is_some_and(|bodies| all_member_bodies_distinct(&bodies))
}

/// Concatenates the bodies of the >=2 sibling top-level definitions the
/// member's trimmed range spans, in source order. Returns `None` when the
/// range covers fewer than two such definitions or a body is unreadable.
fn member_preamble_bodies(snippet: &Snippet<'_>) -> Option<Vec<u8>> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let definitions = top_level_definitions_in_range(tree.root_node(), range);
    if definitions.len() < 2 {
        return None;
    }
    let mut bytes = Vec::new();
    for node in definitions {
        bytes.extend_from_slice(definition_body_bytes(node, snippet.source)?);
    }
    Some(bytes)
}

/// Collects the direct `module` children of a preamble definition kind
/// whose byte span lies fully inside `range`, in source order. Does not
/// recurse, so nested definitions never count toward the run.
fn top_level_definitions_in_range(root: Node<'_>, range: ByteRange) -> Vec<Node<'_>> {
    named_children(root)
        .into_iter()
        .filter(|child| PREAMBLE_KINDS.contains(&child.kind()))
        .filter(|child| child.start_byte() >= range.start && child.end_byte() <= range.end)
        .collect()
}

/// Returns the body bytes of a definition node, descending one level into
/// a `decorated_definition` to reach its wrapped `function_definition`.
fn definition_body_bytes<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    let function = function_node(node)?;
    let body = function.child_by_field_name("body")?;
    source.get(body.start_byte()..body.end_byte())
}

/// Resolves a preamble definition node to its `function_definition`,
/// unwrapping a `decorated_definition` if needed.
fn function_node(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "function_definition" {
        return Some(node);
    }
    named_children(node)
        .into_iter()
        .find(|child| child.kind() == "function_definition")
}

/// Returns true when every member's concatenated body bytes are unique —
/// i.e. no two members are body-equivalent copies of each other.
fn all_member_bodies_distinct(bodies: &[Vec<u8>]) -> bool {
    let unique: BTreeSet<&Vec<u8>> = bodies.iter().collect();
    unique.len() == bodies.len()
}
