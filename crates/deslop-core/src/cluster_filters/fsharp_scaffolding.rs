//! [CLONE-NOISE-FSHARP-IMPORTS] F# import-dominated prologues are scaffolding.
//!
//! Tree-sitter proves the matched range is imports and optional short,
//! structurally distinct implementation. A copied function remains visible.

use tree_sitter::Node;

use super::{
    constant_table::{fsharp_is_container, fsharp_is_trivia, fsharp_module_name},
    language_cluster_shapes, node_intersects_range, parse_for, trimmed_snippet_range, Snippet,
};
use crate::{ast::named_children, ast::ByteRange};

/// A single `open` could be an incidental part of a meaningful match.
const MIN_IMPORT_DECLARATIONS: usize = 2;

/// What implementation shape, if any, accompanies the imports.
enum MemberShape {
    /// The matched range contains only file scaffolding.
    ImportsOnly,
    /// A short function accompanies the imports; this is its expression kind.
    Expression(u16),
}

/// Import lists repeated across F# modules have no extractable implementation.
pub(super) fn is_import_scaffolding_cluster(snippets: &[Snippet<'_>]) -> bool {
    let Some(shapes) = language_cluster_shapes(snippets, "fsharp", member_shape) else {
        return false;
    };
    if shapes
        .iter()
        .all(|shape| matches!(shape, MemberShape::ImportsOnly))
    {
        return true;
    }
    let kinds: Option<Vec<_>> = shapes
        .iter()
        .map(|shape| match shape {
            MemberShape::ImportsOnly => None,
            MemberShape::Expression(kind) => Some(*kind),
        })
        .collect();
    kinds.is_some_and(|kinds| all_unique(&kinds))
}

/// Rejects a member with multiple or substantial implementation nodes.
fn member_shape(snippet: &Snippet<'_>) -> Option<MemberShape> {
    let tree = parse_for(snippet)?;
    let root = tree.root_node();
    (root.kind() == "file").then_some(())?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let (imports, function) = scan_nodes(root, range)?;
    covered_body_shape(function, imports)
}

/// The walker records only the syntax needed for this narrow proof.
fn scan_nodes(root: Node<'_>, range: ByteRange) -> Option<(usize, Option<Node<'_>>)> {
    let mut pending = vec![root];
    let mut imports = 0;
    let mut function = None;
    while let Some(node) = pending.pop() {
        inspect_node(node, range, &mut pending, &mut imports, &mut function)?;
    }
    (imports >= MIN_IMPORT_DECLARATIONS).then_some(())?;
    Some((imports, function))
}

/// A short, fully covered expression can be compared at its root kind.
fn covered_body_shape(function: Option<Node<'_>>, imports: usize) -> Option<MemberShape> {
    let Some(function) = function else {
        return Some(MemberShape::ImportsOnly);
    };
    let body = function_body(function)?;
    body_shorter_than_import_run(body, imports).then_some(MemberShape::Expression(body.kind_id()))
}

/// Module containers can contribute imports and at most one full function.
fn inspect_node<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    pending: &mut Vec<Node<'tree>>,
    imports: &mut usize,
    function: &mut Option<Node<'tree>>,
) -> Option<()> {
    if !node_intersects_range(node, range) {
        return Some(());
    }
    match node.kind() {
        kind if fsharp_is_container(kind) => queue_children(node, range, pending),
        "import_decl" => *imports = (*imports).checked_add(1)?,
        "function_or_value_defn" => record_function(node, range, function)?,
        kind if fsharp_is_trivia(kind) || is_directive(kind) => {}
        _ => return None,
    }
    Some(())
}

/// Module names are headers, not implementation children.
fn queue_children<'tree>(node: Node<'tree>, range: ByteRange, pending: &mut Vec<Node<'tree>>) {
    pending.extend(
        named_children(node).into_iter().filter(|child| {
            node_intersects_range(*child, range) && !fsharp_module_name(node, *child)
        }),
    );
}

/// A second or partially matched function invalidates the shape proof.
fn record_function<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    function: &mut Option<Node<'tree>>,
) -> Option<()> {
    if function.is_some() || node.start_byte() < range.start || node.end_byte() > range.end {
        return None;
    }
    *function = Some(node);
    Some(())
}

/// Only a simple definition with one expression body qualifies.
fn function_body(function: Node<'_>) -> Option<Node<'_>> {
    let children = named_children(function);
    match children.as_slice() {
        [left, body] if left.kind() == "function_declaration_left" => Some(*body),
        _ => None,
    }
}

/// More import statements than implementation lines prove the file match is scaffold-dominated.
fn body_shorter_than_import_run(body: Node<'_>, imports: usize) -> bool {
    body.end_position()
        .row
        .checked_sub(body.start_position().row)
        .and_then(|lines| lines.checked_add(1))
        .is_some_and(|lines| lines < imports)
}

/// A shared expression shape may be a copied implementation, so keep it.
fn all_unique(kinds: &[u16]) -> bool {
    kinds
        .iter()
        .enumerate()
        .all(|(index, kind)| kinds.iter().take(index).all(|previous| previous != kind))
}

/// Compiler and interactive directives have no extractable implementation.
fn is_directive(kind: &str) -> bool {
    matches!(kind, "compiler_directive_decl" | "fsi_directive_decl")
}
