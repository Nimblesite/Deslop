//! Where a wrapper's data goes ([RANK-STRUCTURAL-ONLY-FORWARDING]).
//!
//! The body-shape proof next door says a member's statements *look*
//! declarative. This module says what they *do*, and it is the half that
//! decides whether a finding is erased.
//!
//! # One delegating call proves nothing about the others
//!
//! A wrapper hands data to collaborator state and does nothing else.
//! Asking only whether *some* call in the body delegates reads the one
//! statement that carries no duplication and excuses every other:
//!
//! ```dart
//! final gross = client.fetch(order);
//! return applyMarkup(gross, "standard", 100);
//! ```
//!
//! `client.fetch(order)` delegates and is byte-identical to its
//! sibling's. Everything liftable lives in `applyMarkup`, whose literals
//! are exactly what parameterising it would absorb. The mirror image —
//! `client.submit(normalise(order, "standard", 100))` — computes on the
//! member's own parameter before anything leaves the class at all.
//! Pinned by `dart_forwarding_fail_open::
//! a_same_class_call_after_delegation_is_not_forwarding` and
//! `::a_same_class_call_before_delegation_is_not_forwarding`.
//!
//! # But a call count cannot stand in for it either
//!
//! Requiring a single call convicts the family this filter exists for.
//! Every vendored meilisearch wrapper makes two: `_getTask(http
//! .deleteMethod(route))` wraps the client call in a sibling helper, and
//! `IndexSettings.fromMap(response.data!)` decodes what came back. Both
//! are transport, and `dart_issue_197_single_file_structural_only`
//! requires both stay suppressed.
//!
//! # The line that separates them
//!
//! **Direction of data flow.** A non-delegating call is transport when
//! it consumes only what a delegation already produced — its arguments
//! all derive from the client's response, and it touches that response
//! at all. `_getTask(http.deleteMethod(route))` passes the delegating
//! call itself; `IndexSettings.fromMap(response.data!)` passes the bound
//! response; `response.data!.cast<String>()` takes no arguments and
//! reads the binding. None of them can carry duplicated logic, because
//! none of them is given anything the class chose.
//!
//! A literal, a member parameter, or a bare sibling reference in
//! argument position is the class computing on its own inputs, and that
//! is precisely the parameterisable duplication this tool reports.

use tree_sitter::Node;

use super::{
    first_identifier_bytes, named_non_comment_children, outermost_kind,
    subtree_mentions_identifier, ForwardingGrammar,
};
use crate::{ast::named_children, cluster_filters::node_contains_kind};

/// True when every call the body makes is transport: at least one
/// delegates to collaborator state, and each of the others exists only
/// to consume what a delegation produced.
pub(super) fn body_is_pure_delegation(
    body: Node<'_>,
    container: Node<'_>,
    member: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
) -> bool {
    let collaborators = collaborator_names(container, member, grammar, source);
    let delegated = delegated_names(body, grammar, source, &collaborators);
    let mut calls = Vec::new();
    collect_kinds(body, &[grammar.call], &mut calls);
    calls
        .iter()
        .any(|call| is_delegating_call(*call, grammar, source, &collaborators))
        && calls
            .iter()
            .all(|call| call_is_transport(*call, grammar, source, &collaborators, &delegated))
}

/// A call is transport when it delegates, or when it exists only to
/// consume delegated data: it reaches a delegation at all, and every
/// argument it passes derives from one. The reach test is what rejects a
/// bare `helper()` that reads nothing the client returned.
fn call_is_transport(
    call: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
    collaborators: &[&[u8]],
    delegated: &[&[u8]],
) -> bool {
    if is_delegating_call(call, grammar, source, collaborators) {
        return true;
    }
    derives_from_delegation(call, grammar, source, collaborators, delegated)
        && call_arguments(call, grammar).iter().all(|argument| {
            derives_from_delegation(*argument, grammar, source, collaborators, delegated)
        })
}

/// True when the subtree reaches a delegation: it contains a delegating
/// call, or mentions a name already holding delegated data.
fn derives_from_delegation(
    node: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
    collaborators: &[&[u8]],
    delegated: &[&[u8]],
) -> bool {
    subtree_delegates(node, grammar, source, collaborators)
        || delegated
            .iter()
            .any(|name| subtree_mentions_identifier(node, grammar, source, name))
}

/// The argument expressions of a call, comments removed.
fn call_arguments<'tree>(call: Node<'tree>, grammar: &ForwardingGrammar) -> Vec<Node<'tree>> {
    named_children(call)
        .into_iter()
        .find(|child| child.kind() == grammar.arguments)
        .map_or_else(Vec::new, named_non_comment_children)
}

/// Names that hold data a delegation already produced: a local binding
/// whose initialiser reaches a collaborator, and every parameter of a
/// callback declared inside the body — a lambda handed to a delegated
/// collection receives that collection's elements, so its parameters
/// carry delegated data too. The member's own parameter list sits
/// outside the body and is never collected here.
fn delegated_names<'a>(
    body: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &'a [u8],
    collaborators: &[&[u8]],
) -> Vec<&'a [u8]> {
    let mut declarations = Vec::new();
    collect_kinds(body, &[grammar.parameter], &mut declarations);
    let mut bindings = Vec::new();
    collect_kinds(body, &[grammar.binding], &mut bindings);
    declarations.extend(
        bindings
            .into_iter()
            .filter(|binding| subtree_delegates(*binding, grammar, source, collaborators)),
    );
    declarations
        .into_iter()
        .filter_map(|declaration| first_identifier_bytes(declaration, grammar, source))
        .collect()
}

/// Names the member may legitimately hand data to: the container's
/// instance fields — declarations without a body, so locals inside
/// sibling methods never count — and the member's own parameters.
fn collaborator_names<'a>(
    container: Node<'_>,
    member: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &'a [u8],
) -> Vec<&'a [u8]> {
    let mut declarations = Vec::new();
    collect_field_declarations(container, grammar, &mut declarations);
    if let Some(parameters) = outermost_kind(member, grammar.parameters) {
        collect_kinds(parameters, &[grammar.parameter], &mut declarations);
    }
    declarations
        .into_iter()
        .filter_map(|declaration| first_identifier_bytes(declaration, grammar, source))
        .collect()
}

/// Collects field-declaring nodes: container children with no body of
/// their own, so locals inside sibling methods never count.
fn collect_field_declarations<'tree>(
    container: Node<'tree>,
    grammar: &ForwardingGrammar,
    out: &mut Vec<Node<'tree>>,
) {
    for sibling in named_children(container) {
        if !node_contains_kind(sibling, grammar.body) {
            collect_kinds(sibling, grammar.field_names, out);
        }
    }
}

/// Depth-first collection of every named node whose kind is in `kinds`.
fn collect_kinds<'tree>(node: Node<'tree>, kinds: &[&str], out: &mut Vec<Node<'tree>>) {
    if kinds.contains(&node.kind()) {
        out.push(node);
    }
    for child in named_children(node) {
        collect_kinds(child, kinds, out);
    }
}

/// True when any call in the subtree delegates to a collaborator.
fn subtree_delegates(
    node: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
    collaborators: &[&[u8]],
) -> bool {
    if is_delegating_call(node, grammar, source, collaborators) {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| subtree_delegates(child, grammar, source, collaborators))
}

/// One call delegates when its receiver resolves to a collaborator. A
/// bare helper call, a `this.method(...)` self-call and a static
/// `Type.factory(...)` all fail — their data never leaves the class.
fn is_delegating_call(
    node: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
    collaborators: &[&[u8]],
) -> bool {
    node.kind() == grammar.call
        && member_access_receiver(node, grammar).is_some_and(|receiver| {
            source
                .get(receiver.start_byte()..receiver.end_byte())
                .is_some_and(|name| collaborators.contains(&name))
        })
}

/// The receiver of a member-access call: the callee must be a
/// member-access node of exactly two named children — receiver and
/// member — with an identifier in the receiver position. `this.x(...)`
/// carries one named child (`this` is anonymous) and a chained
/// `a.b.c(...)` nests a member access there, so both return `None`.
fn member_access_receiver<'tree>(
    call: Node<'tree>,
    grammar: &ForwardingGrammar,
) -> Option<Node<'tree>> {
    let first = call.named_child(0)?;
    let unwrapped = if first.kind() == "instantiation_expression" {
        first.named_child(0)?
    } else {
        first
    };
    let callee = Some(unwrapped).filter(|callee| grammar.member_access.contains(&callee.kind()))?;
    match named_children(callee).as_slice() {
        [receiver, _member] if receiver.kind() == grammar.identifier => Some(*receiver),
        _ => None,
    }
}
