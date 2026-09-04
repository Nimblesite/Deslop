//! Bound-result flow for the literal-variation call-sequence classifier.
//!
//! [CLONE-NOISE-LITERAL-VARIATION-CALLS] distinguishes an invariant call
//! carrying reusable logic from an invariant adapter whose result is handed
//! directly to a later varying diagnostic call. This module extracts only the
//! two AST facts that decision needs; it never classifies a cluster itself.

use tree_sitter::Node;

use crate::ast::named_children;

/// Local name a call result is assigned to in its enclosing statement.
pub(super) fn assigned_binding(call: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = call;
    while let Some(parent) = cursor.parent() {
        if matches!(
            parent.kind(),
            "variable_declarator" | "assignment_expression"
        ) {
            let name = parent
                .child_by_field_name("name")
                .or_else(|| parent.child_by_field_name("left"))?;
            return source
                .get(name.start_byte()..name.end_byte())
                .map(<[u8]>::to_vec);
        }
        if is_statement_boundary(parent.kind()) {
            return None;
        }
        cursor = parent;
    }
    None
}

/// Raw identifiers the call consumes: every identifier inside its
/// argument list, plus the arguments of every invocation spelled inside
/// its callee. `expect(generated).toContain("…")` is one call whose
/// callee carries the nested `expect(generated)` invocation, and
/// `generated` flows into the assertion through that receiver exactly as
/// an argument would ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
pub(super) fn consumed_identifiers(call: Node<'_>, source: &[u8], kinds: &[&str]) -> Vec<Vec<u8>> {
    let mut identifiers = Vec::new();
    if let Some(arguments) = argument_list(call) {
        collect_identifiers(arguments, source, &mut identifiers);
    }
    if let Some(callee) = call.child_by_field_name("function") {
        collect_receiver_arguments(callee, source, kinds, &mut identifiers);
    }
    identifiers
}

/// The argument list of a call, under either field name the supported
/// grammars use.
fn argument_list(call: Node<'_>) -> Option<Node<'_>> {
    call.child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
}

/// Collects the argument identifiers of every invocation nested in a
/// callee expression — the receivers a value can flow through.
fn collect_receiver_arguments(
    node: Node<'_>,
    source: &[u8],
    kinds: &[&str],
    out: &mut Vec<Vec<u8>>,
) {
    if kinds.contains(&node.kind()) {
        if let Some(arguments) = argument_list(node) {
            collect_identifiers(arguments, source, out);
        }
    }
    for child in named_children(node) {
        collect_receiver_arguments(child, source, kinds, out);
    }
}

/// Collects identifier leaves without interpreting their language role.
fn collect_identifiers(node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if is_identifier(node.kind()) {
        if let Some(bytes) = source.get(node.start_byte()..node.end_byte()) {
            out.push(bytes.to_vec());
        }
        return;
    }
    for child in named_children(node) {
        collect_identifiers(child, source, out);
    }
}

/// Identifier leaf names used by the supported call grammars.
fn is_identifier(kind: &str) -> bool {
    matches!(kind, "identifier" | "simple_identifier" | "name")
}

/// Boundaries beyond which a call cannot be assigned by the same statement.
fn is_statement_boundary(kind: &str) -> bool {
    kind.ends_with("_statement")
        || matches!(
            kind,
            "lexical_declaration" | "local_variable_declaration" | "variable_declaration"
        )
}
