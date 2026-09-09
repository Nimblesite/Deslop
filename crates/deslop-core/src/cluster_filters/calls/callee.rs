//! Canonical call headers expose receiver-call payloads to the existing
//! literal-variation rule ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).

use tree_sitter::Node;

use super::{
    args::{arg_shape, collect_argument_shapes},
    call_kinds, dataflow, ArgShape, CallShape,
};
use crate::ast::named_children;

/// Terminal syntax nodes have no children.
const LEAF_CHILD_COUNT: usize = 0;

/// One unambiguous part of a callee's syntax and retained names.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum CalleePart {
    /// Syntax kind; receiver identifier spelling is deliberately absent.
    Node(&'static str),
    /// Called names, member selectors and non-payload terminal text.
    Text(Vec<u8>),
    /// A classified string payload, compared through argument positions.
    Payload,
    /// A statement-bearing argument, guarded by the body's own evidence.
    Body,
    /// End of one syntax node, preserving nesting and argument arity.
    End,
}

/// Extracts one call, including arguments belonging to its callee chain.
pub(super) fn call_shape_from_node(
    call: Node<'_>,
    source: &[u8],
    language: &str,
) -> Option<CallShape> {
    let callee_node = call.child_by_field_name("function")?;
    let (mut arguments, mut keywords) = collect_argument_shapes(call, source, language);
    nested_arguments(callee_node, source, language, &mut arguments, &mut keywords);
    Some(CallShape {
        arity: arguments.len(),
        callee: canonical_callee(callee_node, source, language),
        keywords,
        arguments,
        result_binding: dataflow::assigned_binding(call, source),
        consumed_identifiers: dataflow::consumed_identifiers(call, source, call_kinds(language)),
    })
}

/// The receiver's calls contribute payload slots, never sequence steps.
fn nested_arguments(
    node: Node<'_>,
    source: &[u8],
    language: &str,
    arguments: &mut Vec<ArgShape>,
    keywords: &mut Vec<Option<Vec<u8>>>,
) {
    if call_kinds(language).contains(&node.kind()) {
        let (nested, names) = collect_argument_shapes(node, source, language);
        arguments.extend(nested);
        keywords.extend(names);
    }
    for child in named_children(node) {
        nested_arguments(child, source, language, arguments, keywords);
    }
}

/// Reads syntax without making source whitespace part of the header.
fn canonical_callee(node: Node<'_>, source: &[u8], language: &str) -> Vec<CalleePart> {
    let mut parts = Vec::new();
    append_node(node, source, language, &mut parts);
    parts
}

/// Retains syntax and called identities while normalising receiver names.
fn append_node(node: Node<'_>, source: &[u8], language: &str, parts: &mut Vec<CalleePart>) {
    parts.push(CalleePart::Node(node.kind()));
    if matches!(node.kind(), "arguments" | "argument_list") {
        for argument in named_children(node) {
            append_argument(argument, source, language, parts);
        }
    } else if node.child_count() == LEAF_CHILD_COUNT {
        append_terminal(node, source, parts);
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            append_node(child, source, language, parts);
        }
    }
    parts.push(CalleePart::End);
}

/// Uses the same payload classification as each outer argument slot.
fn append_argument(node: Node<'_>, source: &[u8], language: &str, parts: &mut Vec<CalleePart>) {
    match arg_shape(node, source, language) {
        ArgShape::StringLiteral(_, _) => parts.push(CalleePart::Payload),
        ArgShape::Body => parts.push(CalleePart::Body),
        ArgShape::Other => append_node(node, source, language, parts),
    }
}

/// Literal payloads were consumed above; remaining syntax stays exact.
fn append_terminal(node: Node<'_>, source: &[u8], parts: &mut Vec<CalleePart>) {
    let identifier = matches!(
        node.kind(),
        "identifier" | "property_identifier" | "field_identifier" | "type_identifier"
    );
    if !identifier || retained_name(node) {
        if let Some(text) = source.get(node.start_byte()..node.end_byte()) {
            parts.push(CalleePart::Text(text.to_vec()));
        }
    }
}

/// A direct function name or member selector identifies the called code.
fn retained_name(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return true;
    };
    if matches!(parent.kind(), "generic_name" | "generic_function") {
        return retained_name(parent);
    }
    ["function", "property", "field", "attribute", "name"]
        .iter()
        .any(|field| parent.child_by_field_name(field) == Some(node))
}
