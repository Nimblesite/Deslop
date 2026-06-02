//! Module-level constant-table filter for Python.
//!
//! Issue **#133** [CLONE-NOISE-PY-MODULE-CONSTANT-TABLE]: a module that is
//! just a run of module-level `NAME = <literal>` constant assignments — a
//! table of SQL query strings, registry/config values, or similar —
//! normalises to the same structural subtree as any other such table once
//! identifiers, literals, and comments are stripped. Two unrelated tables
//! then reach `structural=1.00, token_jaccard=1.00` and cluster as
//! duplicates, but a table of distinct named constants is data, not
//! extractable logic — there is no shared control flow or abstraction to
//! hoist.
//!
//! Mirrors the Dart const-registry filter (#169) and the module-preamble
//! filter (#104): suppressed **only** when the members differ in raw
//! bytes, so a constants module copied verbatim into two files still
//! surfaces as genuine duplication.

use tree_sitter::Node;

use super::{
    node_intersects_range, parse_for, raw_snippet_texts_differ, trimmed_snippet_range, Snippet,
};

/// Returns true when every cluster member's matched range covers a run of
/// module-level constant assignments (a "constant table") and at least two
/// members differ in raw bytes. Returning true drops the cluster —
/// unrelated data tables are not duplication. A verbatim copy (no byte
/// divergence) falls through and stays visible.
pub(super) fn is_module_constant_table_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && snippets.iter().all(|snippet| snippet.language == "python")
        && snippets.iter().all(covers_only_module_constants)
        && raw_snippet_texts_differ(snippets)
}

/// Returns true when the member's range lies at Python module top level and
/// every top-level statement it covers is a comment, a docstring, or a
/// constant assignment — with at least one constant assignment present.
fn covers_only_module_constants(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    if root.kind() != "module" {
        return false;
    }
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let mut cursor = root.walk();
    let mut saw_constant = false;
    for child in root.named_children(&mut cursor) {
        if !node_intersects_range(child, range) {
            continue;
        }
        match classify_top_level(child) {
            TopLevel::Trivia => {}
            TopLevel::ConstantAssignment => saw_constant = true,
            TopLevel::Other => return false,
        }
    }
    saw_constant
}

/// How a top-level `module` child contributes to the constant-table shape.
enum TopLevel {
    /// A comment or a bare-string docstring — inert framing.
    Trivia,
    /// A `NAME = <literal>` constant assignment.
    ConstantAssignment,
    /// Anything else — imports, defs, classes, control flow, calls. Its
    /// presence means the range is not a pure constant table.
    Other,
}

/// Classifies one top-level `module` child for the constant-table shape.
fn classify_top_level(node: Node<'_>) -> TopLevel {
    match node.kind() {
        "comment" => TopLevel::Trivia,
        "expression_statement" => classify_expression_statement(node),
        _ => TopLevel::Other,
    }
}

/// Classifies an `expression_statement`: a lone docstring is trivia, a
/// constant assignment is a table entry, everything else (including a
/// chained `a = 1; b = 2`) disqualifies the range so it keeps clustering.
fn classify_expression_statement(node: Node<'_>) -> TopLevel {
    let Some(inner) = sole_named_child(node) else {
        return TopLevel::Other;
    };
    match inner.kind() {
        "string" => TopLevel::Trivia,
        "assignment" if assignment_is_constant(inner) => TopLevel::ConstantAssignment,
        _ => TopLevel::Other,
    }
}

/// Returns true when `assignment` binds a bare `NAME` (optionally typed) to
/// a pure literal value — the constant-table entry shape. A target that is
/// an attribute, subscript, or unpacking pattern is mutation, not a
/// constant declaration, and disqualifies the entry.
fn assignment_is_constant(assignment: Node<'_>) -> bool {
    let Some(left) = assignment.child_by_field_name("left") else {
        return false;
    };
    if left.kind() != "identifier" {
        return false;
    }
    assignment
        .child_by_field_name("right")
        .is_some_and(is_constant_value)
}

/// Returns true when `node` is a literal value with no embedded logic: a
/// plain string/number/bool/None, a unary on a literal, or a homogeneous
/// collection of literals. A call, name, attribute, comprehension, lambda,
/// or interpolated f-string is not constant and keeps the cluster visible.
fn is_constant_value(node: Node<'_>) -> bool {
    match node.kind() {
        "string" => string_is_plain(node),
        "integer" | "float" | "true" | "false" | "none" => true,
        "unary_operator" => node
            .child_by_field_name("argument")
            .is_some_and(is_constant_value),
        "parenthesized_expression" => sole_named_child(node).is_some_and(is_constant_value),
        "concatenated_string" | "list" | "tuple" | "set" => {
            all_named_children(node, is_constant_value)
        }
        "dictionary" => all_named_children(node, pair_is_constant),
        _ => false,
    }
}

/// Returns true when a `dictionary` child is a `pair` whose key and value
/// are both constant. A `dictionary_splat` (`**rest`) or any other child
/// disqualifies the dictionary.
fn pair_is_constant(node: Node<'_>) -> bool {
    if node.kind() != "pair" {
        return false;
    }
    let key = node.child_by_field_name("key");
    let value = node.child_by_field_name("value");
    key.is_some_and(is_constant_value) && value.is_some_and(is_constant_value)
}

/// Returns true when `string` carries no `interpolation` child — i.e. it is
/// a plain literal, not an f-string that can embed arbitrary expressions.
fn string_is_plain(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let has_interpolation = node
        .named_children(&mut cursor)
        .any(|child| child.kind() == "interpolation");
    !has_interpolation
}

/// Returns the only named child of `node`, or `None` when it has zero or
/// more than one.
fn sole_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut children = node.named_children(&mut cursor);
    let first = children.next()?;
    if children.next().is_some() {
        return None;
    }
    Some(first)
}

/// Returns true when `predicate` holds for every named child of `node`
/// (vacuously true for an empty collection such as `[]` or `{}`).
fn all_named_children(node: Node<'_>, predicate: fn(Node<'_>) -> bool) -> bool {
    let mut cursor = node.walk();
    let satisfied = node.named_children(&mut cursor).all(predicate);
    satisfied
}
