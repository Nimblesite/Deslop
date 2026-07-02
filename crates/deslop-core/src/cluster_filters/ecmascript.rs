//! JavaScript / TypeScript / TSX false-positive filters.
//!
//! Suppresses clusters whose shape is fixed by a data declaration rather
//! than by extractable duplicate logic, mirroring the Rust struct-field
//! (#224) and Dart class-field (#169) filters for the ECMAScript family.

use tree_sitter::Node;

use super::{
    node_intersects_range, parse_for, raw_snippet_texts_differ, trimmed_snippet_range, Snippet,
};

/// Returns true for a cluster of TypeScript `interface` / object-type data
/// shapes — a run of `property_signature` members (`name: Type;`) — which
/// encode the structure of a value, not extractable logic. Two unrelated
/// interfaces with the same field *shape* but different field *names*
/// normalise to the same skeleton and would otherwise cluster as a top
/// offender, exactly as renamed-field structs did for Rust (#224).
///
/// Guarded like the Rust/Dart field filters: suppressed only when at least
/// two members differ in raw bytes, so a *verbatim* copy-pasted interface
/// still surfaces as genuine duplication rather than being mistaken for two
/// distinct domain types.
pub(super) fn is_ecmascript_data_shape_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && snippets.iter().all(covers_only_property_signatures)
        && raw_snippet_texts_differ(snippets)
}

/// True when the snippet's range is *exactly* a TS `interface` / object-type
/// data shape (optionally export-wrapped) whose every covered member is a
/// `property_signature`. The smallest CST node spanning the range must itself
/// resolve to the data body, so a whole-file or whole-program cluster that
/// merely *contains* a type alongside a real function is never mistaken for a
/// pure data shape — only a logic-free declaration is suppressed.
fn covers_only_property_signatures(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let end_inclusive = range.end.saturating_sub(1).max(range.start);
    let Some(node) = tree
        .root_node()
        .descendant_for_byte_range(range.start, end_inclusive)
    else {
        return false;
    };
    let Some(body) = resolve_data_body(node) else {
        return false;
    };
    let mut cursor = body.walk();
    let mut covered = 0_usize;
    for member in body.named_children(&mut cursor) {
        if !node_intersects_range(member, range) {
            continue;
        }
        if member.kind() != "property_signature" {
            return false;
        }
        covered = covered.saturating_add(1);
    }
    covered >= 1
}

/// Resolves a covering CST node to the `interface_body` / `object_type` it
/// represents, unwrapping a single `export` or declaration wrapper. Returns
/// `None` for anything carrying logic or mixing a data shape with other
/// statements (a `program`, `statement_block`, function, or `export default
/// <value>`), so only a pure data declaration is ever suppressed.
fn resolve_data_body(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "interface_body" | "object_type" => Some(node),
        "interface_declaration" | "type_alias_declaration" | "export_statement" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if let Some(body) = resolve_data_body(child) {
                    return Some(body);
                }
            }
            None
        }
        _ => None,
    }
}
