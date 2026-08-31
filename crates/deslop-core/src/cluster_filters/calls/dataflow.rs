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

/// Raw identifiers used anywhere inside the call's argument list.
pub(super) fn argument_identifiers(call: Node<'_>, source: &[u8]) -> Vec<Vec<u8>> {
    let Some(arguments) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return Vec::new();
    };
    let mut identifiers = Vec::new();
    collect_identifiers(arguments, source, &mut identifiers);
    identifiers
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
