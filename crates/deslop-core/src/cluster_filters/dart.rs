//! Dart-specific false-positive filters.

use tree_sitter::Node;

use super::{
    enclosing_kind, node_contains_kind, node_intersects_range, parse_for, raw_snippet_texts_differ,
    Snippet,
};

/// Returns true for repeated Dart field/const declarations. Field lists
/// inside a class encode data shape, not extractable duplicate logic. This
/// covers both a single field declaration and a run of sibling fields —
/// const data registries such as icon tables, colour palettes, and design
/// tokens (`static const Foo NAME = Foo(<distinct values>);` repeated for
/// hundreds of entries) cluster via sibling-window fingerprints spanning
/// several consecutive declarations, which are un-refactorable data (#169).
///
/// Guarded the same way as the Python module-preamble filter (#104): only
/// suppressed when at least two members differ in raw bytes, so a *verbatim*
/// copy-pasted field block still surfaces as genuine duplication rather than
/// being mistaken for a registry of distinct entries.
pub(super) fn is_dart_class_field_declaration_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && snippets.iter().all(covers_only_field_declarations)
        && raw_snippet_texts_differ(snippets)
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

/// Returns true when a Dart `class_member` declares pure data: it carries a
/// field/const shape (`static const`/`static final` or an initialised
/// instance field) and no executable body. Methods, getters, and setters
/// nest a `function_body`; a field initialised by a closure/lambda nests a
/// `function_expression` (Dart emits no `function_body` for `=> expr` or
/// `(x) { ... }` initialisers). Excluding both keeps logic-bearing fields
/// clustering instead of being mistaken for data (#169 precision).
///
/// Deliberate boundary: a field initialised by a *call* —
/// `IconData(0x...)`, `Color(0xFF...)`, `compute(a, b)` — is treated as
/// data. The registries #169 targets (icon/colour/token tables) are
/// constructor calls, and tree-sitter cannot tell a data constructor from
/// a logic function call without name resolution we do not have. Hiding a
/// run of identical call-shaped initialisers is therefore intentional and
/// consistent with #169; only an embedded `function_expression` body marks
/// a field as logic. The far rarer "table of free-function calls" shape is
/// accepted collateral.
fn is_field_member(member: Node<'_>) -> bool {
    !node_contains_kind(member, "function_body")
        && !node_contains_kind(member, "function_expression")
        && (node_contains_kind(member, "static_final_declaration_list")
            || node_contains_kind(member, "initialized_identifier_list"))
}
