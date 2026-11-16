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

/// [CLONE-NOISE-LITERAL-VARIATION-CALLS-BOUNDED-MEMO] A callee header,
/// held as the digest of its canonical token stream. The header is only
/// ever compared for equality, and a callee can be a whole module — an
/// immediately-invoked wrapper — so holding its tokens made each memoised
/// member cost a copy of the file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct CalleeDigest([u8; blake3::OUT_LEN]);

/// One token of the canonical header stream; each is framed so no two
/// distinct streams hash the same bytes.
#[derive(Clone, Copy)]
enum Token<'a> {
    /// Syntax kind; receiver identifier spelling is deliberately absent.
    Node(&'static str),
    /// Called names, member selectors and non-payload terminal text.
    Text(&'a [u8]),
    /// A classified string payload, compared through argument positions.
    Payload,
    /// A statement-bearing argument, guarded by the body's own evidence.
    Body,
    /// End of one syntax node, preserving nesting and argument arity.
    End,
}

impl Token<'_> {
    /// Writes the token's tag, then its length-prefixed text when it has one.
    fn feed(self, hasher: &mut blake3::Hasher) {
        let (tag, text): (u8, &[u8]) = match self {
            Self::Node(kind) => (0, kind.as_bytes()),
            Self::Text(text) => (1, text),
            Self::Payload => (2, &[]),
            Self::Body => (3, &[]),
            Self::End => (4, &[]),
        };
        let _tagged = hasher.update(&[tag]);
        let _length = hasher.update(&(text.len() as u64).to_le_bytes());
        let _text = hasher.update(text);
    }
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
pub(super) fn canonical_callee(node: Node<'_>, source: &[u8], language: &str) -> CalleeDigest {
    let mut hasher = blake3::Hasher::new();
    append_node(node, source, language, &mut hasher);
    CalleeDigest(*hasher.finalize().as_bytes())
}

/// Retains syntax and called identities while normalising receiver names.
fn append_node(node: Node<'_>, source: &[u8], language: &str, parts: &mut blake3::Hasher) {
    Token::Node(node.kind()).feed(parts);
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
    Token::End.feed(parts);
}

/// Uses the same payload classification as each outer argument slot.
fn append_argument(node: Node<'_>, source: &[u8], language: &str, parts: &mut blake3::Hasher) {
    match arg_shape(node, source, language) {
        // A wrapper's own callee is compared across members by
        // [`super::literal_agreement`], which sees every member at once,
        // so here it needs only the same payload slot a bare string takes.
        ArgShape::StringLiteral(_, _) | ArgShape::LiteralWrapper(_, _) => {
            Token::Payload.feed(parts);
        }
        ArgShape::Body => Token::Body.feed(parts),
        ArgShape::Other => append_node(node, source, language, parts),
    }
}

/// Literal payloads were consumed above; remaining syntax stays exact.
fn append_terminal(node: Node<'_>, source: &[u8], parts: &mut blake3::Hasher) {
    let identifier = matches!(
        node.kind(),
        "identifier" | "property_identifier" | "field_identifier" | "type_identifier"
    );
    if !identifier || is_called_name(node) {
        if let Some(text) = source.get(node.start_byte()..node.end_byte()) {
            Token::Text(text).feed(parts);
        }
    }
}

/// A direct function name or member selector identifies the called code.
/// Shared with [`super::dataflow`], which needs the same distinction from
/// the other side: what the header keeps is what dataflow must not read
/// as a consumed value.
pub(super) fn is_called_name(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return true;
    };
    if matches!(parent.kind(), "generic_name" | "generic_function") {
        return is_called_name(parent);
    }
    ["function", "property", "field", "attribute", "name"]
        .iter()
        .any(|field| parent.child_by_field_name(field) == Some(node))
}
