//! Call-expression shape analysis shared by the language-agnostic
//! literal-variation cluster filter.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - **#70 / #71 / #72** — test-data / scaffolding clusters whose calls
//!   share the same callee but vary in one or more string literal
//!   arguments.

use tree_sitter::Node;

use super::{enclosing_kind, parse_for, Snippet};
use crate::ast::ByteRange;

/// Detects **issues #70 / #71 / #72**: every cluster member is an
/// expression (or expression statement) whose top-level call has the
/// same callee chain across the cluster but **at least one string
/// literal argument differs**.
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
    call_shape_from_node(call, snippet.source)
}

/// Extracts a [`CallShape`] from a concrete call node.
fn call_shape_from_node(call: Node<'_>, source: &[u8]) -> Option<CallShape> {
    let callee_node = call.child_by_field_name("function")?;
    let callee = source
        .get(callee_node.start_byte()..callee_node.end_byte())?
        .to_vec();
    let arguments = collect_argument_shapes(call, source);
    Some(CallShape {
        arity: arguments.len(),
        callee,
        arguments,
    })
}

/// Detects body-range clusters whose contained call sequence has the
/// same callees but intentionally different literal test data.
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
    (0..first.len()).any(|index| sequence_position_differs(&sequences, index))
}

/// Returns every call fully contained in `snippet.range`, preserving
/// source order.
fn call_shapes_in_range(snippet: &Snippet<'_>) -> Option<Vec<CallShape>> {
    let tree = parse_for(snippet)?;
    let mut shapes = Vec::new();
    collect_call_shapes(
        tree.root_node(),
        snippet.range,
        call_kinds(snippet.language),
        snippet.source,
        &mut shapes,
    );
    Some(shapes)
}

/// Recursively collects call nodes within `range`.
fn collect_call_shapes(
    node: Node<'_>,
    range: ByteRange,
    kinds: &[&str],
    source: &[u8],
    out: &mut Vec<CallShape>,
) {
    if node.end_byte() < range.start || node.start_byte() > range.end {
        return;
    }
    if node.start_byte() >= range.start
        && node.end_byte() <= range.end
        && kinds.contains(&node.kind())
    {
        if let Some(shape) = call_shape_from_node(node, source) {
            out.push(shape);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_call_shapes(child, range, kinds, source, out);
    }
}

/// Compares call sequence shape, ignoring literal payloads.
fn same_call_headers(calls: &[CallShape], expected: &[CallShape]) -> bool {
    calls.len() == expected.len()
        && calls
            .iter()
            .zip(expected)
            .all(|(call, base)| call.callee == base.callee && call.arity == base.arity)
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
        _ => &[],
    }
}

/// Walks the named children of the call's `arguments`/`argument_list`
/// node and produces one [`ArgShape`] per argument.
fn collect_argument_shapes(call: Node<'_>, source: &[u8]) -> Vec<ArgShape> {
    let Some(args) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return Vec::new();
    };
    let mut shapes = Vec::new();
    let mut cursor = args.walk();
    for arg in args.named_children(&mut cursor) {
        shapes.push(arg_shape(arg, source));
    }
    shapes
}

/// Classifies one argument node into [`ArgShape`].
fn arg_shape(node: Node<'_>, source: &[u8]) -> ArgShape {
    let inner = unwrap_argument(node);
    if let Some(bytes) = string_literal_bytes(inner, source) {
        return ArgShape::StringLiteral(bytes);
    }
    ArgShape::Other
}

/// Strips a C# `argument` wrapper down to its inner expression so the
/// literal-detection match arms below see the same shapes regardless
/// of language.
fn unwrap_argument(node: Node<'_>) -> Node<'_> {
    if node.kind() == "argument" {
        let mut cursor = node.walk();
        let child = node.named_children(&mut cursor).next();
        if let Some(child) = child {
            return child;
        }
    }
    node
}

/// Returns the bytes of `node` when it is a string-literal-like leaf.
/// Covers Python plain `string`, f-string, and C# `string_literal` /
/// `interpolated_string_expression` so f-string template differences
/// in issue #71 are captured.
fn string_literal_bytes(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let kind = node.kind();
    let is_string = matches!(
        kind,
        "string"
            | "concatenated_string"
            | "string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_expression"
    );
    if !is_string {
        return None;
    }
    source
        .get(node.start_byte()..node.end_byte())
        .map(<[u8]>::to_vec)
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
