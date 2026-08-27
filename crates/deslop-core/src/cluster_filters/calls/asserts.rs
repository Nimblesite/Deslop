//! Assertion admission for the literal-variation sequence rule
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
//!
//! `resp = client.delete(f"/api/…"); assert resp.status_code == 204` is
//! the idiom the filter exists to hide, and the trailing assertion is
//! the test's acceptance criterion on the value the varying call bound
//! — part of the idiom, not authored logic. Admission is deliberately
//! narrow: the statement must be assertion-shaped for its grammar, and
//! every identifier it inspects must name a value one of the covered
//! call-bearing statements bound. An assertion on outside state, a
//! computation smuggled into an assert, or a second call-free statement
//! still blocks the filter, so an authored call-free statement keeps
//! its cluster visible (`rename_needs_an_anchor`).

use tree_sitter::Node;

use super::super::Snippet;

/// Grammars with a call-free assertion statement. Rust (`assert!` is a
/// macro invocation), C#, and ECMAScript spell assertions as calls, so
/// the sequence rule already governs them position by position.
const fn assert_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["assert_statement"],
        _ => &[],
    }
}

/// True when `statement` is an assertion whose every subject identifier
/// names a value bound by one of `call_statements` — and it inspects at
/// least one such value, so a vacuous assertion does not qualify.
pub(super) fn is_assert_on_call_bound_value(
    statement: Node<'_>,
    call_statements: &[&Node<'_>],
    snippet: &Snippet<'_>,
) -> bool {
    if !assert_kinds(snippet.language).contains(&statement.kind()) {
        return false;
    }
    let bound = bound_names(call_statements, snippet.source);
    let mut subjects = Vec::new();
    collect_subject_identifiers(statement, snippet.source, &mut subjects);
    !subjects.is_empty() && subjects.iter().all(|name| bound.contains(name))
}

/// Names the call-bearing statements bind: every assignment-target
/// identifier they contain.
fn bound_names(statements: &[&Node<'_>], source: &[u8]) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    for statement in statements {
        collect_assignment_targets(**statement, source, &mut names);
    }
    names
}

/// Walks `node` recording the identifiers on the `left` of every
/// assignment it contains, chained assignments included.
fn collect_assignment_targets(node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left") {
            collect_identifiers(left, source, out);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_assignment_targets(child, source, out);
    }
}

/// Records every identifier in `node`'s subtree.
fn collect_identifiers(node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if node.kind() == "identifier" {
        if let Some(bytes) = source.get(node.start_byte()..node.end_byte()) {
            out.push(bytes.to_vec());
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_identifiers(child, source, out);
    }
}

/// Records the identifiers `node` inspects — every identifier except an
/// attribute name, which names a field on a subject rather than a
/// subject (`resp.status_code` inspects `resp`).
fn collect_subject_identifiers(node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if node.kind() == "identifier" {
        if let Some(bytes) = source.get(node.start_byte()..node.end_byte()) {
            out.push(bytes.to_vec());
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !is_attribute_name(node, child) {
            collect_subject_identifiers(child, source, out);
        }
    }
}

/// True when `child` sits in `parent`'s attribute-name field.
fn is_attribute_name(parent: Node<'_>, child: Node<'_>) -> bool {
    parent.kind() == "attribute"
        && parent
            .child_by_field_name("attribute")
            .is_some_and(|name| name.id() == child.id())
}
