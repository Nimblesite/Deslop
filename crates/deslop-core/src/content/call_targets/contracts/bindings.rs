//! [FUSED-CONTENT-GATE-CALL-TARGET-CONTRACT] Guard evidence follows one unchanged binding.

use super::{bytes, NormalizedNode, IDENTIFIER_KIND};

/// These productions introduce a scope for the declarations they contain.
const BINDING_SCOPES: &[&str] = &[
    "statement_block",
    "function_declaration",
    "function_expression",
    "arrow_function",
    "catch_clause",
    "for_statement",
    "for_in_statement",
];
/// Their first child is the binding being declared, rather than its initializer.
const BINDING_DECLARATIONS: &[&str] = &[
    "variable_declarator",
    "required_parameter",
    "optional_parameter",
    "catch_clause",
    "function_declaration",
    "class_declaration",
];
/// These expressions can replace the checked receiver before its selected call.
const RECEIVER_WRITES: &[&str] = &[
    "assignment_expression",
    "augmented_assignment_expression",
    "update_expression",
];
/// Each child in these patterns is itself a binding, never a property selection.
const COMPOSITE_PATTERNS: &[&str] = &["array_pattern", "object_pattern", "rest_pattern"];
/// Expressions can bind their own callable name or an unparenthesized parameter.
const LOCAL_CALLABLE_BINDINGS: &[&str] = &["function_expression", "arrow_function"];

/// Every enclosing lexical scope must also contain the predicate call.
pub(super) fn declaration_reaches_call(
    declaration: &NormalizedNode,
    call: &NormalizedNode,
    nodes: &[&NormalizedNode],
) -> bool {
    nodes.iter().all(|scope| {
        !BINDING_SCOPES.contains(&scope.kind)
            || scope.byte_range == declaration.byte_range
            || !scope.byte_range.covers(declaration.byte_range)
            || scope.byte_range.covers(call.byte_range)
    })
}

/// Any competing copied binding makes the predicate's identity ambiguous.
pub(super) fn names_binding(node: &NormalizedNode, name: &NormalizedNode, source: &[u8]) -> bool {
    (BINDING_DECLARATIONS.contains(&node.kind)
        || LOCAL_CALLABLE_BINDINGS.contains(&node.kind)
        || RECEIVER_WRITES.contains(&node.kind))
        && node
            .children
            .first()
            .is_some_and(|pattern| binds_receiver(pattern, name, source))
}

/// A guard cannot describe a replacement value or a different lexical binding.
pub(super) fn preserves_receiver(
    branch: &NormalizedNode,
    call: &NormalizedNode,
    receiver: &NormalizedNode,
    source: &[u8],
) -> bool {
    let mut pending = vec![(branch, true)];
    while let Some((node, visible)) = pending.pop() {
        if node.byte_range.start >= call.byte_range.start {
            continue;
        }
        if invalidates_receiver(node, call, receiver, source, visible) {
            return false;
        }
        let visible = visible && scope_contains_call(node, call);
        pending.extend(node.children.iter().map(|child| (child, visible)));
    }
    true
}

/// A sibling scope's local declarations cannot shadow the selected call.
fn scope_contains_call(node: &NormalizedNode, call: &NormalizedNode) -> bool {
    !BINDING_SCOPES.contains(&node.kind) || node.byte_range.covers(call.byte_range)
}

/// A completed write changes the value; an enclosing declaration changes its binding.
fn invalidates_receiver(
    node: &NormalizedNode,
    call: &NormalizedNode,
    receiver: &NormalizedNode,
    source: &[u8],
    visible: bool,
) -> bool {
    let writes =
        RECEIVER_WRITES.contains(&node.kind) && node.byte_range.end <= call.byte_range.start;
    let declares = visible && BINDING_DECLARATIONS.contains(&node.kind);
    (writes || declares)
        && node
            .children
            .first()
            .is_some_and(|pattern| binds_receiver(pattern, receiver, source))
}

/// Inspect assignment patterns, excluding property keys and member-access receivers.
fn binds_receiver(pattern: &NormalizedNode, receiver: &NormalizedNode, source: &[u8]) -> bool {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        if pattern.kind == IDENTIFIER_KIND && bytes(pattern, source) == bytes(receiver, source) {
            return true;
        }
        extend_bindings(pattern, &mut pending);
    }
    false
}

/// A pair's value binds; defaults and parentheses preserve their first binding.
fn extend_bindings<'node>(
    pattern: &'node NormalizedNode,
    pending: &mut Vec<&'node NormalizedNode>,
) {
    if COMPOSITE_PATTERNS.contains(&pattern.kind) {
        pending.extend(pattern.children.iter());
    } else {
        let binding = match pattern.kind {
            "pair_pattern" => pattern.children.last(),
            "assignment_pattern" | "object_assignment_pattern" | "parenthesized_expression" => {
                pattern.children.first()
            }
            _ => None,
        };
        pending.extend(binding);
    }
}
