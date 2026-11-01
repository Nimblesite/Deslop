//! Go fixture-driven tests ([CLONE-NOISE-GO-FIXTURE-TEST]).
//!
//! A Go integration test often binds its whole scenario to one string, hands
//! that to the harness, and asserts on the output. What the test does is in
//! the string, so two such tests with different strings share only the
//! harness calls. A test holding anything beyond those calls fails the proof
//! and the cluster stays.

use tree_sitter::Node;

use super::{
    data_rows::members_are_copies, enclosing_kind, is_multi_member_language_cluster,
    node_search::node_enclosed_by_range, parse_for, raw_snippet_texts_differ,
    trimmed_snippet_range, Snippet,
};
use crate::ast::named_children;

/// The language this filter reads.
const LANGUAGE: &str = "go";
/// The prefix Go requires of a test function's name.
const TEST_PREFIX: &[u8] = b"Test";
/// The node a test is declared by.
const FUNCTION: [&str; 1] = ["function_declaration"];
/// The string literal kinds a fixture can be.
const FIXTURE_KINDS: [&str; 2] = ["raw_string_literal", "interpreted_string_literal"];

/// Returns true when every member is one whole fixture-driven test and the
/// members hold different fixtures.
pub(super) fn is_fixture_test_cluster(snippets: &[Snippet<'_>]) -> bool {
    is_multi_member_language_cluster(snippets, LANGUAGE)
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(is_fixture_test)
        && !members_are_copies(snippets)
}

/// True when the snippet is a whole `Test…` function of the fixture form.
fn is_fixture_test(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    enclosing_kind(tree.root_node(), range, &FUNCTION).is_some_and(|function| {
        is_test_name(function, snippet.source)
            && function.child_by_field_name("body").is_some_and(|body| {
                node_enclosed_by_range(body, range) && has_fixture_body(&statements(body))
            })
    })
}

/// True when the function's name opens with `Test`.
fn is_test_name(function: Node<'_>, source: &[u8]) -> bool {
    function
        .child_by_field_name("name")
        .and_then(|name| source.get(name.start_byte()..name.end_byte()))
        .is_some_and(|name| name.starts_with(TEST_PREFIX))
}

/// The statements of a block, through the grammar's statement list.
fn statements(body: Node<'_>) -> Vec<Node<'_>> {
    let children = named_children(body);
    match children.as_slice() {
        [only] if only.kind() == "statement_list" => named_children(*only),
        _ => children,
    }
}

/// One string binding, then only harness calls.
fn has_fixture_body(body: &[Node<'_>]) -> bool {
    let Some((first, rest)) = body.split_first() else {
        return false;
    };
    binds_string(*first)
        && !rest.is_empty()
        && rest.iter().all(|statement| is_harness_step(*statement))
}

/// True for `files := <string>`.
fn binds_string(statement: Node<'_>) -> bool {
    bound_value(statement).is_some_and(|value| FIXTURE_KINDS.contains(&value.kind()))
}

/// The single value a short variable declaration binds.
fn bound_value(statement: Node<'_>) -> Option<Node<'_>> {
    if statement.kind() != "short_var_declaration" {
        return None;
    }
    let values = named_children(statement.child_by_field_name("right")?);
    match values.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// `b := harness(t, files)` or `b.Assert("path", "text")`.
fn is_harness_step(statement: Node<'_>) -> bool {
    let call = bound_value(statement).or_else(|| match statement.kind() {
        "expression_statement" => named_children(statement).first().copied(),
        "call_expression" => Some(statement),
        _ => None,
    });
    call.is_some_and(|call| call.kind() == "call_expression" && has_plain_arguments(call))
}

/// True when a call's arguments are all names or literals.
fn has_plain_arguments(call: Node<'_>) -> bool {
    call.child_by_field_name("arguments")
        .is_some_and(|arguments| {
            named_children(arguments).into_iter().all(|argument| {
                argument.kind() == "identifier"
                    || FIXTURE_KINDS.contains(&argument.kind())
                    || matches!(argument.kind(), "true" | "false" | "nil" | "int_literal")
            })
        })
}
