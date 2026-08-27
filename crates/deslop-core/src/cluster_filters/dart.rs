//! Dart-specific false-positive filters.

use tree_sitter::Node;

use crate::ast::named_children;

use super::{
    enclosing_kind, node_intersects_range, parse_for, raw_snippet_texts_differ, ParseCache, Snippet,
};

/// Superclass markers of Flutter's mandated widget-declaration scaffold
/// ([CLONE-NOISE-DART-WIDGET-SCAFFOLD]). Every `StatefulWidget`
/// must declare its own `createState`, every `StatelessWidget` its own
/// `build`, and every state class extends `State<T>` — none of it can
/// be extracted or merged, so a cluster of such declarations must never
/// rank as actionable duplication.
const WIDGET_SUPERCLASS_MARKERS: &[&str] = &["StatelessWidget", "StatefulWidget", "State<"];

/// Returns true when every member of a Dart cluster covers only whole
/// widget-scaffold class declarations ([CLONE-NOISE-DART-WIDGET-SCAFFOLD]).
/// Containment is deliberate in both directions: a member must contain
/// its classes entirely (so a subtree *inside* a widget body — the
/// actual copy-pasted logic — never matches), and every top-level node
/// the member touches must be such a class (so a window mixing a free
/// function with a widget shell keeps clustering as logic). Body-level
/// clones keep surfacing as their own subtree clusters, which is what
/// makes hiding the shells recall-safe.
pub(super) fn is_dart_widget_scaffold_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2 && snippets.iter().all(covers_only_widget_scaffold_classes)
}

/// True when the snippet covers ≥1 widget-scaffold class and nothing else.
fn covers_only_widget_scaffold_classes(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    let mut covered_classes = 0_usize;
    for child in named_children(root) {
        if !node_intersects_range(child, snippet.range) {
            continue;
        }
        if is_contained_widget_scaffold_class(child, snippet) {
            covered_classes = covered_classes.saturating_add(1);
            continue;
        }
        if is_contained_main_launcher(child, snippet) {
            continue;
        }
        return false;
    }
    covered_classes >= 1
}

/// True when `node` is fully inside the snippet range and is (part of)
/// the mandated Flutter app launcher — `void main() => runApp(…)`. The
/// Dart grammar splits a top-level function into signature and body
/// siblings, so both halves are recognised by their launcher markers.
fn is_contained_main_launcher(node: Node<'_>, snippet: &Snippet<'_>) -> bool {
    if node.start_byte() < snippet.range.start || node.end_byte() > snippet.range.end {
        return false;
    }
    node.utf8_text(snippet.source)
        .is_ok_and(|text| text.starts_with("void main(") || text.contains("runApp("))
}

/// True when `node` is a class declaration fully inside the snippet
/// range whose superclass names a Flutter widget marker.
fn is_contained_widget_scaffold_class(node: Node<'_>, snippet: &Snippet<'_>) -> bool {
    if node.kind() != "class_definition"
        || node.start_byte() < snippet.range.start
        || node.end_byte() > snippet.range.end
    {
        return false;
    }
    named_children(node).into_iter().any(|child| {
        child.kind() == "superclass"
            && child
                .utf8_text(snippet.source)
                .is_ok_and(|text| WIDGET_SUPERCLASS_MARKERS.iter().any(|m| text.contains(m)))
    })
}

/// Returns true for repeated Dart field/const declarations. Field lists
/// inside a class encode data shape, not extractable duplicate logic. This
/// covers both a single field declaration and a run of sibling fields —
/// const data registries such as icon tables, colour palettes, and design
/// tokens (`static const Foo NAME = Foo(<distinct values>);` repeated for
/// hundreds of entries) cluster via sibling-window fingerprints spanning
/// several consecutive declarations, which are un-refactorable data.
///
/// Guarded the same way as the Python module-preamble filter: only
/// suppressed when at least two members differ in raw bytes, so a *verbatim*
/// copy-pasted field block still surfaces as genuine duplication rather than
/// being mistaken for a registry of distinct entries.
pub(super) fn is_dart_class_field_declaration_cluster(
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> bool {
    snippets.len() >= 2
        && snippets
            .iter()
            .all(|snippet| covers_only_field_declarations(snippet, cache))
        && raw_snippet_texts_differ(snippets)
}

/// Returns true when the snippet's range sits inside a Dart class body and
/// every class member it covers is a field/const declaration. Method,
/// getter, and setter members carry a `function_body`, so a snippet that
/// covers any of them falls through and keeps clustering.
fn covers_only_field_declarations(snippet: &Snippet<'_>, cache: &ParseCache) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    let Some(body) = enclosing_kind(root, snippet.range, &["class_body"]) else {
        return false;
    };
    let mut covered = 0_usize;
    for member in named_children(body) {
        if !node_intersects_range(member, snippet.range) {
            continue;
        }
        if !is_field_member(member, snippet.file_id, cache) {
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
/// clustering instead of being mistaken for data.
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
fn is_field_member(member: Node<'_>, file_id: crate::state::FileId, cache: &ParseCache) -> bool {
    let kinds = cache.dart_field_kinds(file_id, member);
    !kinds.has_body()
        && !kinds.has_function_expression()
        && (kinds.has_static_final_list() || kinds.has_initialized_identifier_list())
}
