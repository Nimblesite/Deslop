//! Dart-specific false-positive filters.

use tree_sitter::Node;

use super::{enclosing_kind, node_contains_kind, node_intersects_range, parse_for, Snippet};

/// Returns true for repeated Dart field/const declarations. Field lists
/// inside a class encode data shape, not extractable duplicate logic. This
/// covers both a single field declaration and a run of sibling fields —
/// const data registries such as icon tables, colour palettes, and design
/// tokens (`static const Foo NAME = Foo(<distinct values>);` repeated for
/// hundreds of entries) cluster via sibling-window fingerprints spanning
/// several consecutive declarations, which are un-refactorable data (#169).
pub(super) fn is_dart_class_field_declaration_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2 && snippets.iter().all(covers_only_field_declarations)
}

/// Returns true when the snippet's range sits inside a Dart class body and
/// every class member it covers is a field/const declaration. Method,
/// getter, and setter members carry a `function_body`, so a snippet that
/// covers any of them falls through and keeps clustering.
fn covers_only_field_declarations(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    let Some(body) = enclosing_kind(root, snippet.range, &["class_body"]) else {
        return false;
    };
    let mut cursor = body.walk();
    let mut covered = 0_usize;
    for member in body.named_children(&mut cursor) {
        if !node_intersects_range(member, snippet.range) {
            continue;
        }
        if !is_field_member(member) {
            return false;
        }
        covered = covered.saturating_add(1);
    }
    covered >= 1
}

/// Returns true when a Dart `class_member` declares data: it carries a
/// field/const shape (`static const`/`static final` or an initialised
/// instance field) and no executable body. Methods, getters, and setters
/// all nest a `function_body`, so they are never treated as fields.
fn is_field_member(member: Node<'_>) -> bool {
    !node_contains_kind(member, "function_body")
        && (node_contains_kind(member, "static_final_declaration_list")
            || node_contains_kind(member, "initialized_identifier_list"))
}
