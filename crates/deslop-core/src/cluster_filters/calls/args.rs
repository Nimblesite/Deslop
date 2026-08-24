//! Per-argument shape extraction for the literal-variation call filter
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]): classifying each argument
//! node into the [`ArgShape`] summary the parent module compares across
//! cluster members. Split from the parent, which owns the call-shape
//! types and the cluster-level rules.

use tree_sitter::Node;

use super::super::constant_table::is_literal_value;
use super::ArgShape;

/// Walks the named children of the call's `arguments`/`argument_list`
/// node and produces one [`ArgShape`] per argument.
pub(super) fn collect_argument_shapes(
    call: Node<'_>,
    source: &[u8],
    language: &str,
) -> (Vec<ArgShape>, Vec<Option<Vec<u8>>>) {
    let Some(args) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return (Vec::new(), Vec::new());
    };
    let mut shapes = Vec::new();
    let mut keywords = Vec::new();
    let mut cursor = args.walk();
    for arg in args.named_children(&mut cursor) {
        shapes.push(arg_shape(arg, source, language));
        keywords.push(keyword_name(arg, source));
    }
    (shapes, keywords)
}

/// Classifies one argument node into [`ArgShape`].
fn arg_shape(node: Node<'_>, source: &[u8], language: &str) -> ArgShape {
    let inner = unwrap_argument(node);
    if let Some(bytes) = string_literal_bytes(inner, source) {
        return ArgShape::StringLiteral(bytes);
    }
    if let Some(bytes) = literal_collection_bytes(inner, source, language) {
        return ArgShape::StringLiteral(bytes);
    }
    ArgShape::Other
}

/// Raw bytes of an argument that is a **pure literal collection
/// carrying text** — `["a", "b"]`, `{ kind: "record", width: 4 }`,
/// `("alpha", 1)`. Such an argument is test data passed inline, exactly
/// like a bare string literal, and reading it as an opaque `Other`
/// blinded the filter to the only position a family varied in
/// (gh #284/#285: `buildSchema({ kind: "record", … })` per scenario).
///
/// Deliberately **not** every literal. A bare number is how a real clone
/// spells the one parameter it should have been given —
/// `applyDiscount(0.1)` against `applyDiscount(0.2)` is a clone worth
/// reporting, not scaffolding — so a payload qualifies only when it is a
/// collection *and* it carries at least one string. Purity is
/// [`is_literal_value`], the same predicate
/// [CLONE-NOISE-CONSTANT-TABLE] uses, so an element that is a call or a
/// name disqualifies the whole argument.
fn literal_collection_bytes(node: Node<'_>, source: &[u8], language: &str) -> Option<Vec<u8>> {
    let is_collection = matches!(
        node.kind(),
        "list"
            | "tuple"
            | "set"
            | "dictionary"
            | "array"
            | "object"
            | "array_expression"
            | "tuple_expression"
    );
    if !is_collection || !is_literal_value(language, node) || !carries_string_leaf(node) {
        return None;
    }
    source
        .get(node.start_byte()..node.end_byte())
        .map(<[u8]>::to_vec)
}

/// True when the subtree holds at least one string-literal leaf.
fn carries_string_leaf(node: Node<'_>) -> bool {
    if string_literal_bytes(node, &[]).is_some() || is_string_kind(node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
    children.into_iter().any(carries_string_leaf)
}

/// Strips a C# `argument` wrapper, or a Python `keyword_argument`'s
/// `name=` prefix, down to the inner expression so the
/// literal-detection match arms below see the same shapes regardless of
/// language.
///
/// The keyword case is gh #103's third miss-class: every call site of an
/// already-extracted helper passes its payload by keyword
/// (`_post_turn(client, key, message="…", conversation_id=None)`), so
/// every varying literal sat behind a `keyword_argument` node and the
/// filter measured *no* string arguments at all. The keyword **name**
/// does not travel with the value — [`keyword_name`] captures it into
/// the call header instead, so `f(alpha="x")` and `f(beta="x")` stay
/// different call shapes rather than reading as one shape with a varying
/// literal.
fn unwrap_argument(node: Node<'_>) -> Node<'_> {
    if node.kind() == "argument" {
        let mut cursor = node.walk();
        let child = node.named_children(&mut cursor).next();
        if let Some(child) = child {
            return child;
        }
    }
    if node.kind() == "keyword_argument" {
        if let Some(value) = node.child_by_field_name("value") {
            return value;
        }
    }
    node
}

/// The keyword an argument is passed under, when it is passed by
/// keyword at all. Part of the call *header*, never of its payload: two
/// calls that name different parameters are two different call shapes,
/// whatever literals they carry.
fn keyword_name(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    if node.kind() != "keyword_argument" {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    source
        .get(name.start_byte()..name.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns the bytes of `node` when it is a string-literal-like leaf.
/// Covers Python plain `string`, f-string, and C# `string_literal` /
/// `interpolated_string_expression` so f-string template differences
/// count as literal variation.
fn string_literal_bytes(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    if !is_string_kind(node.kind()) {
        return None;
    }
    source
        .get(node.start_byte()..node.end_byte())
        .map(<[u8]>::to_vec)
}

/// Tree-sitter node kinds that spell a string literal in one of the
/// supported grammars, including the interpolated forms whose template
/// text is itself the varying payload.
fn is_string_kind(kind: &str) -> bool {
    matches!(
        kind,
        "string"
            | "concatenated_string"
            | "string_literal"
            | "raw_string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_expression"
            | "template_string"
    )
}
