//! Dart-specific false-positive filters.

use tree_sitter::Node;

use super::{
    node_intersects_range,
    node_search::{covered_children_satisfy, KindSearch},
    parse_for, raw_snippet_texts_differ, ParseCache, Snippet,
};
use crate::ast::named_children;

/// Framework superclass markers ([CLONE-NOISE-DART-WIDGET-SCAFFOLD]).
/// These identify potential shells; authored logic must still pass the body check.
const WIDGET_SUPERCLASS_MARKERS: &[&str] = &["StatelessWidget", "StatefulWidget", "State"];

/// A closed set of construction-only body nodes. Computation, control flow,
/// closures and unknown syntax cannot establish scaffolding.
const SCAFFOLD_BODY_KINDS: &[&str] = &[
    "function_body",
    "block",
    "return_statement",
    "call_expression",
    "member_expression",
    "arguments",
    "named_argument",
    "label",
    "identifier",
    "type",
    "type_identifier",
    "type_arguments",
    "instantiation_expression",
    "list_literal",
    "decimal_integer_literal",
    "decimal_floating_point_literal",
    "true",
    "false",
    "null_literal",
    "string_literal",
    "string_literal_single_quotes",
    "string_literal_double_quotes",
    "template_chars_single_single",
    "template_chars_double_single",
];

/// Returns true when every member of a Dart cluster covers only whole
/// construction-only widget classes ([CLONE-NOISE-DART-WIDGET-SCAFFOLD]).
/// Containment is deliberate in both directions: a member must contain
/// its classes entirely (so a subtree *inside* a widget body — the
/// actual copy-pasted logic — never matches), and every top-level node
/// the member touches must be such a class (so a window mixing a free
/// function with a widget shell keeps clustering as logic). Classes containing
/// authored logic do not qualify, so their copied bodies remain reportable.
pub(super) fn is_dart_widget_scaffold_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(covers_only_widget_scaffold_classes)
        && !has_copied_build_body(snippets)
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

/// Recognises only a complete `main` declaration calling `runApp` once.
fn is_contained_main_launcher(node: Node<'_>, snippet: &Snippet<'_>) -> bool {
    if node.start_byte() < snippet.range.start || node.end_byte() > snippet.range.end {
        return false;
    }
    if node.kind() != "function_declaration"
        || declaration_name(node, snippet.source) != Some(b"main")
    {
        return false;
    }
    node.child_by_field_name("body")
        .and_then(launcher_call)
        .is_some_and(|call| {
            call.child_by_field_name("function")
                .filter(|callee| callee.kind() == "identifier")
                .and_then(|callee| snippet.source.get(callee.byte_range()))
                == Some(b"runApp")
                && body_is_scaffold(call)
        })
}

/// Unwraps only a single call, rejecting additional statements or expressions.
fn launcher_call(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "call_expression" => Some(node),
        "function_body" | "block" | "expression_statement" => {
            let children = named_children(node);
            let [child] = children.as_slice() else {
                return None;
            };
            launcher_call(*child)
        }
        _ => None,
    }
}

/// True when `node` is a class declaration fully inside the snippet
/// range whose superclass names a Flutter widget marker.
fn is_contained_widget_scaffold_class(node: Node<'_>, snippet: &Snippet<'_>) -> bool {
    if node.kind() != "class_declaration"
        || node.start_byte() < snippet.range.start
        || node.end_byte() > snippet.range.end
    {
        return false;
    }
    is_widget_superclass(node, snippet.source) && has_only_scaffold_bodies(node, snippet)
}

/// Matches the parsed superclass identifier exactly, without substring matches.
fn is_widget_superclass(node: Node<'_>, source: &[u8]) -> bool {
    node.child_by_field_name("superclass")
        .and_then(|superclass| superclass.child_by_field_name("type"))
        .and_then(|kind| kind.named_child(0))
        .filter(|name| name.kind() == "type_identifier")
        .and_then(|name| source.get(name.byte_range()))
        .is_some_and(|name| {
            WIDGET_SUPERCLASS_MARKERS
                .iter()
                .any(|marker| name == marker.as_bytes())
        })
}

/// A whole group survives only when every member shares the same copied layout.
fn has_copied_build_body(snippets: &[Snippet<'_>]) -> bool {
    let Some(first) = snippets.first().and_then(widget_layout_key) else {
        return false;
    };
    snippets
        .iter()
        .all(|snippet| widget_layout_key(snippet).as_ref() == Some(&first))
}

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] The full ordered layout of a proven scaffold.
pub(super) fn widget_layout_key(snippet: &Snippet<'_>) -> Option<Vec<Vec<u8>>> {
    if !covers_only_widget_scaffold_classes(snippet) {
        return None;
    }
    let bodies = build_body_bytes(snippet)?;
    (!bodies.is_empty()).then_some(bodies)
}

/// Collects complete `build` bodies contained in the reported range.
fn build_body_bytes(snippet: &Snippet<'_>) -> Option<Vec<Vec<u8>>> {
    let tree = parse_for(snippet)?;
    KindSearch::enclosed(snippet.range, |kind| kind == "method_declaration")
        .nodes(tree.root_node())
        .into_iter()
        .filter(|method| declaration_name(*method, snippet.source) == Some(b"build"))
        .map(|method| {
            let body = method.child_by_field_name("body")?;
            snippet.source.get(body.byte_range()).map(<[u8]>::to_vec)
        })
        .collect()
}

/// Reads the declared name through a function or method signature.
fn declaration_name<'src>(node: Node<'_>, source: &'src [u8]) -> Option<&'src [u8]> {
    let signature = node.child_by_field_name("signature")?;
    let function = if signature.kind() == "method_signature" {
        signature.named_child(0)?
    } else {
        signature
    };
    let name = function.child_by_field_name("name")?;
    source.get(name.byte_range())
}

/// Every covered executable body must prove construction-only scaffolding.
fn has_only_scaffold_bodies(node: Node<'_>, snippet: &Snippet<'_>) -> bool {
    KindSearch::enclosed(snippet.range, |kind| {
        matches!(kind, "function_body" | "function_expression")
    })
    .nodes(node)
    .into_iter()
    .all(body_is_scaffold)
}

/// Rejects authored computation and unknown syntax in an iterative AST walk.
fn body_is_scaffold(body: Node<'_>) -> bool {
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        if node.is_extra() {
            continue;
        }
        if !SCAFFOLD_BODY_KINDS.contains(&node.kind()) {
            return false;
        }
        pending.extend(named_children(node));
    }
    true
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
    covered_children_satisfy(snippet, &["class_body"], |member| {
        is_field_member(member, snippet.file_id, cache)
    })
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
