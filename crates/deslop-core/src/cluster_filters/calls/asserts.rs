//! Assertion admission for the literal-variation sequence rule
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
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
use crate::ast::named_children;

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
/// least one such value, so a vacuous assertion does not qualify
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
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
    collect_identifiers(statement, snippet.source, subject_child, &mut subjects);
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
            collect_identifiers(left, source, every_child, out);
        }
    }
    for child in named_children(node) {
        collect_assignment_targets(child, source, out);
    }
}

/// Decides whether the collector descends from `parent` into `child`.
type ChildGuard = fn(Node<'_>, Node<'_>) -> bool;

/// The whole subtree counts: an assignment target names what it binds
/// wherever inside the target expression the identifier sits.
fn every_child(_parent: Node<'_>, _child: Node<'_>) -> bool {
    true
}

/// Attribute names are skipped: they name a field *on* a subject rather
/// than a subject, so `resp.status_code` inspects `resp` alone.
fn subject_child(parent: Node<'_>, child: Node<'_>) -> bool {
    let names_a_field = parent.kind() == "attribute"
        && parent
            .child_by_field_name("attribute")
            .is_some_and(|name| name.id() == child.id());
    !names_a_field
}

/// Records every identifier in `node`'s subtree that `descend` admits.
fn collect_identifiers(node: Node<'_>, source: &[u8], descend: ChildGuard, out: &mut Vec<Vec<u8>>) {
    if node.kind() == "identifier" {
        if let Some(bytes) = source.get(node.start_byte()..node.end_byte()) {
            out.push(bytes.to_vec());
        }
        return;
    }
    for child in named_children(node) {
        if descend(node, child) {
            collect_identifiers(child, source, descend, out);
        }
    }
}
