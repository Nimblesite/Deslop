use tree_sitter::Node;

use super::super::{node_intersects_range, parse_for, trimmed_snippet_range, Snippet};
use crate::ast::{named_children, ByteRange};

/// Returns true when `snippet` contains the
/// `.iter().map(|x| x.field.method(...)).collect()` chain at any depth.
pub(super) fn snippet_contains_iter_collect_idiom(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = match trimmed_snippet_range(snippet) {
        Some(range) => range,
        None => snippet.range,
    };
    find_iter_collect_idiom(tree.root_node(), range, snippet.source)
}

/// Recursively scans `node` (restricted to `range`) for the idiom.
fn find_iter_collect_idiom(node: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    if !node_intersects_range(node, range) {
        return false;
    }
    if node.kind() == "call_expression" && call_is_iter_collect_idiom(node, source) {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| find_iter_collect_idiom(child, range, source))
}

/// Returns true when `call` is a `.collect()` / `.collect::<...>()` call
/// whose receiver is `.iter().map(<single-method-closure>)`.
fn call_is_iter_collect_idiom(call: Node<'_>, source: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let Some(receiver) = receiver_for_method(function, source, "collect") else {
        return false;
    };
    map_call_has_field_method_closure(receiver, source)
}

/// Returns the receiver expression when `function` is a `field_expression`
/// or `generic_function` selecting the named method (e.g. `collect` or
/// `collect::<Vec<_>>`).
fn receiver_for_method<'tree>(
    function: Node<'tree>,
    source: &[u8],
    name: &str,
) -> Option<Node<'tree>> {
    let field_expression = if function.kind() == "generic_function" {
        function.child_by_field_name("function")?
    } else {
        function
    };
    if field_expression.kind() != "field_expression" {
        return None;
    }
    let field = field_expression.child_by_field_name("field")?;
    if field.kind() != "field_identifier" {
        return None;
    }
    let field_bytes = source.get(field.start_byte()..field.end_byte())?;
    if field_bytes != name.as_bytes() {
        return None;
    }
    field_expression.child_by_field_name("value")
}

/// Returns true when `receiver` is a `<expr>.iter().map(|x| body)` call
/// whose closure body is a single `x.field.method(...)` access.
fn map_call_has_field_method_closure(receiver: Node<'_>, source: &[u8]) -> bool {
    if receiver.kind() != "call_expression" {
        return false;
    }
    let Some(map_callee) = receiver.child_by_field_name("function") else {
        return false;
    };
    let Some(iter_receiver) = receiver_for_method(map_callee, source, "map") else {
        return false;
    };
    if !receiver_chain_has_iter_call(iter_receiver, source) {
        return false;
    }
    let Some(map_args) = receiver.child_by_field_name("arguments") else {
        return false;
    };
    closure_body_is_field_method_call(map_args, source)
}

/// Returns true when `expr` is itself a `.iter()` call expression with
/// no arguments — that's the start of the idiom chain.
fn receiver_chain_has_iter_call(expr: Node<'_>, source: &[u8]) -> bool {
    if expr.kind() != "call_expression" {
        return false;
    }
    let Some(function) = expr.child_by_field_name("function") else {
        return false;
    };
    receiver_for_method(function, source, "iter").is_some()
}

/// Returns true when the call's `arguments` is exactly one closure whose
/// body is a `closure_arg.field.method(...)` call expression.
fn closure_body_is_field_method_call(arguments: Node<'_>, source: &[u8]) -> bool {
    let closures: Vec<Node<'_>> = named_children(arguments)
        .into_iter()
        .filter(|child| child.kind() == "closure_expression")
        .collect();
    let [closure] = closures.as_slice() else {
        return false;
    };
    let Some(closure_arg) = sole_closure_parameter_bytes(*closure, source) else {
        return false;
    };
    let Some(body) = closure.child_by_field_name("body") else {
        return false;
    };
    closure_body_matches_field_method(body, source, closure_arg)
}

/// Returns the byte range of the single identifier parameter declared
/// by the closure, or `None` when the closure has zero / multiple / non
/// trivial parameters (we only suppress single-arg field projections).
fn sole_closure_parameter_bytes<'a>(closure: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    let parameters = closure.child_by_field_name("parameters")?;
    let named = named_children(parameters);
    let [first] = named.as_slice() else {
        return None;
    };
    if first.kind() != "identifier" {
        return None;
    }
    source.get(first.start_byte()..first.end_byte())
}

/// Returns true when `body` is exactly `<closure_arg>.<field>.<method>(...)`.
fn closure_body_matches_field_method(body: Node<'_>, source: &[u8], closure_arg: &[u8]) -> bool {
    if body.kind() != "call_expression" {
        return false;
    }
    let Some(function) = body.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "field_expression" {
        return false;
    }
    let Some(method_field) = function.child_by_field_name("field") else {
        return false;
    };
    if method_field.kind() != "field_identifier" {
        return false;
    }
    let Some(field_value) = function.child_by_field_name("value") else {
        return false;
    };
    field_expression_projects_closure_arg(field_value, source, closure_arg)
}

/// Returns true when `node` is `<closure_arg>.<field>` (one field hop).
fn field_expression_projects_closure_arg(
    node: Node<'_>,
    source: &[u8],
    closure_arg: &[u8],
) -> bool {
    if node.kind() != "field_expression" {
        return false;
    }
    let Some(base) = node.child_by_field_name("value") else {
        return false;
    };
    if base.kind() != "identifier" {
        return false;
    }
    let base_bytes = source.get(base.start_byte()..base.end_byte());
    if base_bytes != Some(closure_arg) {
        return false;
    }
    node.child_by_field_name("field")
        .is_some_and(|field| field.kind() == "field_identifier")
}
