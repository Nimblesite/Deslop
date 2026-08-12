//! Chained nested-dict assertion filter
//! ([CLONE-NOISE-PY-DICT-ASSERT]).
//!
//! `assert payload["k1"]["k2"] == V` over a locally-built literal dict is
//! the pytest idiom for checking a nested response shape. Identifier and
//! literal normalisation collapses the variable, the keys and the
//! expected value, so two tests that verify entirely unrelated contracts
//! — a PATCH response and an `OpenAPI` document — reduce to the same
//! `assert __var__[__str__][__str__] == __const__` skeleton and cluster
//! across files.
//!
//! # Every granularity of the same idiom
//!
//! Fingerprinting emits one subtree per AST node, so the idiom is offered
//! for suppression at several ranges: the assert run alone, the enclosing
//! `test_*` function, and the whole module. They are not
//! interchangeable — cross-cluster subsumption only collapses views that
//! cover the same region in both directions, so the module-wide view
//! survives on its own whenever it names a different file set than the
//! assert-run view.
//!
//! Recognising only the innermost range therefore hid the idiom and
//! published it at the same time: two unrelated pytest modules surfaced
//! as a whole-file `structural_only` duplicate while their assert runs
//! were correctly suppressed. Matching on the `test_*` functions the
//! range *intersects* — enclosing or enclosed — sees one idiom at every
//! depth. Pinned by
//! `python_issue_107::chained_dict_assertions_across_test_files_do_not_cluster`.

use tree_sitter::Node;

use super::{
    is_multi_member_language_cluster, node_intersects_range, parse_for,
    python::python_function_name_starts_with, raw_snippet_texts_differ, spans_multiple_files,
    trimmed_snippet_range, Snippet,
};
use crate::ast::ByteRange;

/// Detects [CLONE-NOISE-PY-DICT-ASSERT]: the chained
/// `assert <var>[k1][k2] == V` shape across at least two unrelated
/// pytest test functions in different files.
///
/// Members whose reported bytes are all identical are exempt: a verbatim
/// copy of a test is real duplication whatever idiom it is written in,
/// and this filter exists for tests that merely *rhyme*.
pub(super) fn is_chained_dict_assert_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "python") {
        return false;
    }
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(is_chained_dict_assert_snippet)
}

/// Returns true when every `test_*` function the reported range touches
/// asserts only chained-subscript lookups over literal payloads, and the
/// range touches at least one.
fn is_chained_dict_assert_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(range) = trimmed_snippet_range(snippet) else {
        return false;
    };
    let mut functions = Vec::new();
    collect_intersecting_functions(tree.root_node(), range, &mut functions);
    !functions.is_empty()
        && functions
            .iter()
            .all(|function| function_is_chained_dict_test(*function, range, snippet.source))
}

/// Collects every `function_definition` whose bytes overlap `range` —
/// the function enclosing a statement-level range, and the functions
/// contained in a function- or module-level one.
fn collect_intersecting_functions<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    out: &mut Vec<Node<'tree>>,
) {
    if !node_intersects_range(node, range) {
        return;
    }
    if node.kind() == "function_definition" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_intersecting_functions(child, range, out);
    }
}

/// Returns true for a pytest `test_*` function whose body, within
/// `range`, is only chained-dict assertions and the literal payloads
/// they read.
fn function_is_chained_dict_test(function: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    python_function_name_starts_with(function, source, b"test_")
        && function
            .child_by_field_name("body")
            .is_some_and(|body| body_has_only_chained_dict_asserts(body, range, source))
}

/// Returns true when every named child of `body` overlapping `range` is
/// either a chained-dict `assert_statement` or the literal-payload
/// assignment such an assert reads — and at least one assert is present.
///
/// The payload assignment belongs to the idiom. `data = {...}` followed
/// by asserts into `data` is one construct; excluding the assignment
/// recognised the idiom only when the reported range happened to start
/// after it.
fn body_has_only_chained_dict_asserts(body: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    let mut cursor = body.walk();
    let mut saw = false;
    for child in body.named_children(&mut cursor) {
        if !node_intersects_range(child, range) {
            continue;
        }
        if is_literal_payload_assignment(child) {
            continue;
        }
        if child.kind() != "assert_statement" || !assert_statement_is_chained_dict(child, source) {
            return false;
        }
        saw = true;
    }
    saw
}

/// Returns true for `<name> = { ... }` — the literal dict the chained
/// assertions read. Only a dictionary literal counts: a call, a fixture
/// reference or a comprehension is program logic, not test payload.
fn is_literal_payload_assignment(statement: Node<'_>) -> bool {
    let mut cursor = statement.walk();
    statement.kind() == "expression_statement"
        && statement.named_children(&mut cursor).any(|child| {
            child.kind() == "assignment"
                && child
                    .child_by_field_name("right")
                    .is_some_and(|right| right.kind() == "dictionary")
        })
}

/// Returns true for `assert <chain> == <const>` / `is <const>` where the
/// chain is two or more nested subscript accesses against an identifier.
fn assert_statement_is_chained_dict(assert_node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = assert_node.walk();
    let mut named = assert_node.named_children(&mut cursor);
    let Some(first) = named.next() else {
        return false;
    };
    let chain = match first.kind() {
        "comparison_operator" => comparison_left(first),
        _ => Some(first),
    };
    let Some(chain) = chain else { return false };
    subscript_chain_depth(chain, source) >= 2
}

/// Returns the left operand of a Python `comparison_operator` node.
fn comparison_left(comparison: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = comparison.walk();
    let first = comparison.named_children(&mut cursor).next();
    first
}

/// Counts subscript hops down a `subscript(subscript(identifier))` tower.
fn subscript_chain_depth(node: Node<'_>, source: &[u8]) -> usize {
    if node.kind() != "subscript" {
        return 0;
    }
    let Some(value) = node.child_by_field_name("value") else {
        return 0;
    };
    if value.kind() == "subscript" {
        return subscript_chain_depth(value, source).saturating_add(1);
    }
    if value.kind() == "identifier" && source.get(value.start_byte()..value.end_byte()).is_some() {
        return 1;
    }
    0
}
