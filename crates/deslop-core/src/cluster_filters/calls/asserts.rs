//! Assertion admission for the literal-variation sequence rule
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
//!
//! `resp = client.delete(f"/api/…"); assert resp.status_code == 204` is
//! the idiom the filter exists to hide, and the trailing assertion is
//! the test's acceptance criterion on the value the varying call bound
//! — part of the idiom, not authored logic. Admission is deliberately
//! narrow: the statement must be assertion-shaped for its grammar, and
//! every identifier it inspects must name a value one of the covered
//! call-bearing statements bound. An assertion on outside state, or a
//! computation smuggled into an assert, still blocks the filter, so an
//! authored call-free statement keeps its cluster visible
//! (`rename_needs_an_anchor`).
//!
//! One second call-free statement joins it: the literal tautology
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]),
//! `explicit_host_id = "fly-1"; assert explicit_host_id == "fly-1"`,
//! which asserts a value against itself and so tests nothing. Building
//! the value instead of writing it down — `host_prefix + "1"` — is
//! authored data handling and keeps blocking.

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
    let bound = bound_names(call_statements, snippet.source);
    is_assert_on(statement, &bound, snippet)
}

/// True when the two call-free statements are a literal tautology
/// followed by the assertion that reads it: the first writes a literal
/// into a local nothing else in the range touches, and the second is an
/// assertion admitted once that local counts as bound
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]).
pub(super) fn is_literal_tautology_pair(
    pair: [&Node<'_>; 2],
    call_statements: &[&Node<'_>],
    covered: &[Node<'_>],
    snippet: &Snippet<'_>,
) -> bool {
    let [tautology, assertion] = pair;
    let Some(target) = literal_assignment_target(*tautology, snippet.source) else {
        return false;
    };
    if !only_the_assertion_reads(&target, *tautology, *assertion, covered, snippet.source) {
        return false;
    }
    let mut bound = bound_names(call_statements, snippet.source);
    bound.push(target);
    is_assert_on(*assertion, &bound, snippet)
}

/// True when `statement` is an assertion whose subject identifiers are
/// non-empty and all named in `bound`.
fn is_assert_on(statement: Node<'_>, bound: &[Vec<u8>], snippet: &Snippet<'_>) -> bool {
    if !assert_kinds(snippet.language).contains(&statement.kind()) {
        return false;
    }
    let mut subjects = Vec::new();
    collect_identifiers(statement, snippet.source, subject_child, &mut subjects);
    !subjects.is_empty() && subjects.iter().all(|name| bound.contains(name))
}

/// Right-hand sides that are a literal outright: a string, number,
/// boolean, or `None`. Python spellings — no other grammar has a
/// call-free assertion statement, so no other grammar reaches here.
const LITERAL_KINDS: &[&str] = &["string", "integer", "float", "true", "false", "none"];

/// How often the tautology's local may be named across the covered
/// statements: written once, read once by the assertion.
const TAUTOLOGY_OCCURRENCES: usize = 2;

/// The local a `name = <literal>` statement binds. `None` for anything
/// else — a computed right-hand side, an interpolated string, a
/// destructuring target, or a statement that is not an assignment.
fn literal_assignment_target(statement: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let assignment = sole_assignment(statement)?;
    let left = assignment.child_by_field_name("left")?;
    let right = assignment.child_by_field_name("right")?;
    let is_literal = left.kind() == "identifier"
        && LITERAL_KINDS.contains(&right.kind())
        && names_in(right, source).is_empty();
    is_literal
        .then(|| source.get(left.start_byte()..left.end_byte()))
        .flatten()
        .map(<[u8]>::to_vec)
}

/// The one assignment a statement is, whether the grammar wraps it in an
/// expression statement or not. `None` when the statement holds anything
/// besides a single assignment.
fn sole_assignment(statement: Node<'_>) -> Option<Node<'_>> {
    if statement.kind() == "assignment" {
        return Some(statement);
    }
    match named_children(statement).as_slice() {
        [only] if only.kind() == "assignment" => Some(*only),
        _ => None,
    }
}

/// True when `target` is named exactly twice across `covered` — once in
/// the tautology that writes it, once in the assertion that reads it —
/// so no other covered statement consumes the value.
fn only_the_assertion_reads(
    target: &[u8],
    tautology: Node<'_>,
    assertion: Node<'_>,
    covered: &[Node<'_>],
    source: &[u8],
) -> bool {
    let mentions = |node| {
        names_in(node, source)
            .iter()
            .filter(|n| *n == target)
            .count()
    };
    let total: usize = covered.iter().map(|node| mentions(*node)).sum();
    total == TAUTOLOGY_OCCURRENCES && mentions(tautology) == 1 && mentions(assertion) == 1
}

/// Every identifier in `node`'s subtree, attribute names included: a
/// name reached by any route is a mention, and counting a field access
/// as one only ever blocks the filter.
fn names_in(node: Node<'_>, source: &[u8]) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    collect_identifiers(node, source, every_child, &mut names);
    names
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
