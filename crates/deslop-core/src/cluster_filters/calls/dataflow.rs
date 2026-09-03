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

/// Raw identifiers the call consumes: everything inside its argument
/// list, plus everything its **receiver** names.
///
/// A receiver is part of a callee ([CLONE-NOISE-LITERAL-VARIATION-CALLS],
/// gh #284), and it is also a value the call reads:
/// `expect(generated).toContain("…")` consumes `generated` as surely as
/// `assertContains(generated, "…")` would. Reading the argument list
/// alone lost that, so a scenario family whose adapter result reaches the
/// varying assertions only through their receivers looked like shared
/// authored logic and blocked its own suppression.
pub(super) fn consumed_identifiers(call: Node<'_>, source: &[u8]) -> Vec<Vec<u8>> {
    let mut identifiers = Vec::new();
    if let Some(arguments) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    {
        collect_identifiers(arguments, source, &mut identifiers);
    }
    collect_receiver_identifiers(call, source, &mut identifiers);
    identifiers
}

/// Adds the identifiers of the callee's receiver. A bare-identifier
/// callee is a function name and has no receiver, so it contributes
/// nothing; otherwise the receiver is the callee's first named child —
/// the expression the member is selected from, whatever the grammar
/// calls that field — and every identifier inside it is consumed.
fn collect_receiver_identifiers(call: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    let Some(callee) = call.child_by_field_name("function") else {
        return;
    };
    if is_identifier(callee.kind()) {
        return;
    }
    let Some(receiver) = named_children(callee).into_iter().next() else {
        return;
    };
    collect_identifiers(receiver, source, out);
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
