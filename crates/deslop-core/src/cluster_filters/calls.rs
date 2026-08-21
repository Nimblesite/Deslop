//! Call-expression shape analysis shared by the language-agnostic
//! literal-variation cluster filter
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).

use tree_sitter::Node;

use super::{constant_table::is_literal_value, enclosing_kind, parse_for, Snippet};
use crate::ast::ByteRange;

/// Detects literal-variation call scaffolding
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]): every cluster member
/// resolves to the same callee/arity call shape — one enclosing call,
/// or the same ordered call sequence — with **at least one string
/// literal argument differing** across members.
pub(super) fn is_literal_variation_call_cluster(snippets: &[Snippet<'_>]) -> bool {
    let calls: Option<Vec<CallShape>> = snippets.iter().map(call_shape).collect();
    if is_literal_variation_call_set(calls) {
        return true;
    }
    is_literal_variation_call_sequence(snippets)
}

/// Applies the literal-variation rule to one comparable call per
/// cluster member.
fn is_literal_variation_call_set(calls: Option<Vec<CallShape>>) -> bool {
    let Some(calls) = calls else { return false };
    let Some(first) = calls.first() else {
        return false;
    };
    if !calls.iter().all(|call| call.callee == first.callee) {
        return false;
    }
    if !calls.iter().all(|call| call.arity == first.arity) {
        return false;
    }
    if !calls.iter().all(|call| call.keywords == first.keywords) {
        return false;
    }
    has_differing_string_literals(&calls)
}

/// Distilled view of a call expression used to compare cluster members.
#[derive(Clone)]
struct CallShape {
    /// Concrete callee string. Captured from raw source so identifier
    /// text the normalised AST collapses is preserved.
    callee: Vec<u8>,
    /// Number of arguments to the call.
    arity: usize,
    /// Keyword each argument is passed under, positionally, `None` for
    /// positional arguments. Part of the header: two calls naming
    /// different parameters are different shapes ([`keyword_name`]).
    keywords: Vec<Option<Vec<u8>>>,
    /// Per-argument summary used for literal-variation detection.
    arguments: Vec<ArgShape>,
}

/// Per-argument summary recorded for each call.
#[derive(Clone)]
enum ArgShape {
    /// Raw bytes of a string-literal argument (or string content of an
    /// f-string / interpolated string).
    StringLiteral(Vec<u8>),
    /// Anything else — non-string literal, identifier, sub-expression.
    Other,
}

/// Extracts the [`CallShape`] for the call expression covering
/// `snippet.range`. Returns `None` when no call is present.
fn call_shape(snippet: &Snippet<'_>) -> Option<CallShape> {
    let tree = parse_for(snippet)?;
    let call = enclosing_kind(
        tree.root_node(),
        snippet.range,
        call_kinds(snippet.language),
    )?;
    call_shape_from_node(call, snippet.source, snippet.language)
}

/// Extracts a [`CallShape`] from a concrete call node.
fn call_shape_from_node(call: Node<'_>, source: &[u8], language: &str) -> Option<CallShape> {
    let callee_node = call.child_by_field_name("function")?;
    let callee = source
        .get(callee_node.start_byte()..callee_node.end_byte())?
        .to_vec();
    let (arguments, keywords) = collect_argument_shapes(call, source, language);
    Some(CallShape {
        arity: arguments.len(),
        callee,
        keywords,
        arguments,
    })
}

/// Detects body-range clusters whose contained call sequence has the
/// same callees but intentionally different literal test data.
///
/// Every position must vary. A sequence in which some calls carry
/// differing literals while others are invariant is not payload — the
/// invariant calls are shared logic the members genuinely duplicate, and
/// hiding the cluster would lose a real Type-2 clone. Two `[Fact]` tests
/// that fetch different URLs and then run the same four assertions are
/// the case this distinguishes: one varying call, four invariant ones.
/// Scaffolding has nothing left once the literals are removed.
fn is_literal_variation_call_sequence(snippets: &[Snippet<'_>]) -> bool {
    let sequences: Option<Vec<Vec<CallShape>>> =
        snippets.iter().map(call_shapes_in_range).collect();
    let Some(sequences) = sequences else {
        return false;
    };
    let Some(first) = sequences.first() else {
        return false;
    };
    if first.is_empty() || !sequences.iter().all(|seq| same_call_headers(seq, first)) {
        return false;
    }
    (0..first.len()).all(|index| sequence_position_differs(&sequences, index))
}

/// Returns every call fully contained in `snippet.range`, preserving
/// source order.
fn call_shapes_in_range(snippet: &Snippet<'_>) -> Option<Vec<CallShape>> {
    let tree = parse_for(snippet)?;
    let mut shapes = Vec::new();
    let walk = Walk {
        range: snippet.range,
        kinds: call_kinds(snippet.language),
        source: snippet.source,
        language: snippet.language,
    };
    collect_call_shapes(tree.root_node(), &walk, &mut shapes);
    Some(shapes)
}

/// Recursively collects call nodes within `range`.
///
/// A call recorded here does **not** contribute its own callee
/// expression to the sequence again. `expect(generated).toContain("…")`
/// is one call whose callee happens to be spelled with a nested
/// `expect(generated)` invocation; counting the receiver as an
/// independent sequence position made the sequence read as
/// `[expect, expect(...).toContain]`, and since the receiver carries no
/// literal it could never vary — so the "every position must vary" rule
/// refused a family that varies in the only place it has (gh #284). The
/// receiver's bytes are already inside [`CallShape::callee`], so the
/// information is not lost, only counted once. Arguments are still
/// walked: a call passed *as* an argument is genuinely a separate call.
fn collect_call_shapes(node: Node<'_>, walk: &Walk<'_>, out: &mut Vec<CallShape>) {
    if node.end_byte() < walk.range.start || node.start_byte() > walk.range.end {
        return;
    }
    let recorded = node.start_byte() >= walk.range.start
        && node.end_byte() <= walk.range.end
        && walk.kinds.contains(&node.kind());
    if recorded {
        if let Some(shape) = call_shape_from_node(node, walk.source, walk.language) {
            out.push(shape);
            walk_argument_children(node, walk, out);
            return;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_call_shapes(child, walk, out);
    }
}

/// Everything the recursive collector needs that does not change as it
/// descends: the window it may record inside, the call kinds of the
/// language, the raw source, and the language itself.
struct Walk<'a> {
    /// The reported byte window; a call must sit wholly inside it.
    range: ByteRange,
    /// Tree-sitter kinds that count as a call in this language.
    kinds: &'a [&'a str],
    /// Raw source bytes of the member's file.
    source: &'a [u8],
    /// Language id, for literal-payload classification.
    language: &'a str,
}

/// Continues the walk inside a recorded call's argument list only,
/// leaving its callee expression out of the sequence.
fn walk_argument_children(call: Node<'_>, walk: &Walk<'_>, out: &mut Vec<CallShape>) {
    let Some(args) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return;
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        collect_call_shapes(child, walk, out);
    }
}

/// Compares call sequence shape, ignoring literal payloads.
fn same_call_headers(calls: &[CallShape], expected: &[CallShape]) -> bool {
    calls.len() == expected.len()
        && calls
            .iter()
            .zip(expected)
            .all(|(call, base)| {
                call.callee == base.callee
                    && call.arity == base.arity
                    && call.keywords == base.keywords
            })
}

/// Returns true when `index` has intentional literal variation across
/// all call sequences.
fn sequence_position_differs(sequences: &[Vec<CallShape>], index: usize) -> bool {
    let calls: Vec<CallShape> = sequences
        .iter()
        .filter_map(|sequence| sequence.get(index).cloned())
        .collect();
    calls.len() == sequences.len() && has_differing_string_literals(&calls)
}

/// Returns the set of tree-sitter node kinds that count as call
/// expressions per language.
const fn call_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["call"],
        b"csharp" => &["invocation_expression"],
        b"rust" => &["call_expression", "macro_invocation"],
        // Dart `call_expression` exposes a `function` field; the
        // `constructor_invocation` node does not, so it is intentionally
        // excluded from literal-variation comparison.
        b"dart" => &["call_expression"],
        // gh #284/#285: ECMAScript was absent from this map entirely, so
        // the filter could not fire for **any** JavaScript/TypeScript
        // cluster however plainly it was literal-variation scaffolding —
        // seven independent codec diagnostics sharing one
        // `expectErrorMessages` helper rendered `nearly_identical` at
        // `fused 0.86`, and a run of `expect(x).toContain("…")` lines
        // rendered `identical` at `1.00`. The grammar's call node is
        // `call_expression` with the same `function` / `arguments` fields
        // the other languages expose.
        b"javascript" | b"typescript" | b"tsx" => &["call_expression"],
        _ => &[],
    }
}

/// Walks the named children of the call's `arguments`/`argument_list`
/// node and produces one [`ArgShape`] per argument.
fn collect_argument_shapes(
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
        "list" | "tuple" | "set" | "dictionary" | "array" | "object" | "array_expression"
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

/// Returns true when at least one positional argument index has
/// differing string-literal bytes across the cluster. Non-string
/// arguments are ignored — the heuristic only fires when the
/// distinguishing variation is in literal text.
fn has_differing_string_literals(calls: &[CallShape]) -> bool {
    let Some(first) = calls.first() else {
        return false;
    };
    let mut saw_difference = false;
    let mut saw_string_arg = false;
    for index in 0..first.arguments.len() {
        let Some(ArgShape::StringLiteral(baseline)) = first.arguments.get(index) else {
            continue;
        };
        saw_string_arg = true;
        for call in calls.iter().skip(1) {
            match call.arguments.get(index) {
                Some(ArgShape::StringLiteral(bytes)) if bytes != baseline => {
                    saw_difference = true;
                }
                Some(ArgShape::StringLiteral(_)) => {}
                _ => {
                    return false;
                }
            }
        }
    }
    saw_string_arg && saw_difference
}
