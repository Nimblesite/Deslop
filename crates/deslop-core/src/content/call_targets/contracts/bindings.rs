//! [FUSED-CONTENT-GATE-CALL-TARGET-CONTRACT] Guard evidence follows one unchanged binding.

use super::{bytes, NormalizedNode, IDENTIFIER_KIND};

/// A declaration's name belongs to its parent scope, unlike an expression's name.
const FUNCTION_DECLARATION: &str = "function_declaration";
/// These productions introduce a scope for the declarations they contain.
const BINDING_SCOPES: &[&str] = &[
    "statement_block",
    FUNCTION_DECLARATION,
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
    FUNCTION_DECLARATION,
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
/// These containers declare bindings directly in their own lexical scope.
const LOCAL_BINDING_CONTAINERS: &[&str] = &["lexical_declaration", "formal_parameters"];

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
    if LOCAL_BINDING_CONTAINERS.contains(&node.kind) {
        return node
            .children
            .iter()
            .any(|child| names_binding(child, name, source));
    }
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
        if unrelated_binding(node, call, receiver, source) {
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

/// A separate lexical binding owns its assignments, including those in callbacks.
fn unrelated_binding(
    node: &NormalizedNode,
    call: &NormalizedNode,
    receiver: &NormalizedNode,
    source: &[u8],
) -> bool {
    BINDING_SCOPES.contains(&node.kind)
        && !node.byte_range.covers(call.byte_range)
        && !(node.kind == FUNCTION_DECLARATION && names_binding(node, receiver, source))
        && scope_binds_receiver(node, receiver, source)
}

/// Stop at nested scopes: their declarations cannot own a sibling scope's writes.
fn scope_binds_receiver(scope: &NormalizedNode, receiver: &NormalizedNode, source: &[u8]) -> bool {
    if LOCAL_CALLABLE_BINDINGS.contains(&scope.kind) && names_binding(scope, receiver, source) {
        return true;
    }
    let mut pending: Vec<_> = scope.children.iter().collect();
    while let Some(node) = pending.pop() {
        if LOCAL_BINDING_CONTAINERS.contains(&node.kind) && names_binding(node, receiver, source) {
            return true;
        }
        if !BINDING_SCOPES.contains(&node.kind) {
            pending.extend(node.children.iter());
        }
    }
    false
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
    let declares = visible
        && (BINDING_DECLARATIONS.contains(&node.kind)
            || (LOCAL_CALLABLE_BINDINGS.contains(&node.kind)
                && node.byte_range.covers(call.byte_range)));
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
