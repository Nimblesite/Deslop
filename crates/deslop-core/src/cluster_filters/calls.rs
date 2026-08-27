//! Call-expression shape analysis shared by the language-agnostic
//! literal-variation cluster filter
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).

use tree_sitter::Node;

use std::sync::Arc;

use super::{enclosing_kind, parse_for, snippets::CallSequence, ParseCache, Snippet};
use crate::ast::{named_children, ByteRange};

use args::collect_argument_shapes;

/// Per-argument shape extraction for the filter.
mod args;

/// Assertion admission for the covered-statement rule.
mod asserts;

/// Detects literal-variation call scaffolding
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]): every cluster member
/// resolves to the same callee/arity call shape — one enclosing call,
/// or the same ordered call sequence — with **at least one string
/// literal argument differing** across members.
pub(super) fn is_literal_variation_call_cluster(
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> bool {
    let calls: Option<Vec<Arc<CallShape>>> = snippets
        .iter()
        .map(|snippet| cache.call_shape(snippet, || call_shape(snippet)))
        .collect();
    if is_literal_variation_call_set(calls) {
        return true;
    }
    is_literal_variation_call_sequence(snippets, cache)
}

/// Applies the literal-variation rule to one comparable call per
/// cluster member.
fn is_literal_variation_call_set(calls: Option<Vec<Arc<CallShape>>>) -> bool {
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
    has_differing_string_literals(calls.iter().map(std::convert::AsRef::as_ref))
}

/// Distilled view of a call expression used to compare cluster members.
#[derive(Clone)]
pub(crate) struct CallShape {
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
fn is_literal_variation_call_sequence(snippets: &[Snippet<'_>], cache: &ParseCache) -> bool {
    let cells: Option<Vec<Arc<CallSequence>>> = snippets
        .iter()
        .map(|snippet| cache.call_sequence(snippet, || Some(call_sequence(snippet))))
        .collect();
    let Some(cells) = cells else {
        return false;
    };
    if !cells.iter().all(|cell| cell.statements_admissible) {
        return false;
    }
    let sequences: Option<Vec<&[CallShape]>> =
        cells.iter().map(|cell| cell.shapes.as_deref()).collect();
    sequences.is_some_and(|sequences| every_sequence_position_varies(&sequences))
}

/// True when the members share one non-empty ordered call header and
/// every position in it carries differing string literals.
fn every_sequence_position_varies(sequences: &[&[CallShape]]) -> bool {
    let Some(first) = sequences.first() else {
        return false;
    };
    if first.is_empty() || !sequences.iter().all(|seq| same_call_headers(seq, first)) {
        return false;
    }
    (0..first.len()).all(|index| sequence_position_differs(sequences, index))
}

/// Computes the fused literal-variation sequence cell for one snippet:
/// the covered-statement flag and the in-range call sequence, both pure
/// functions of `(file, range)` and memoised together
/// ([PERF-FLUTTER-TODO-CORPUS]).
fn call_sequence(snippet: &Snippet<'_>) -> super::snippets::CallSequence {
    super::snippets::CallSequence {
        statements_admissible: covered_statements_admissible(snippet),
        shapes: call_shapes_in_range(snippet),
    }
}

/// Whether the statements covered by `snippet` are admissible to the
/// sequence rule: every complete covered statement contains a call,
/// except that one lone call-free statement is admitted when it is an
/// assertion on a value the covered calls bound — the trailing
/// acceptance check of the test idiom this filter hides (gh #70, #71).
///
/// Anything else call-free blocks the filter. A varying call is not the
/// whole matched region when an adjacent authored statement carries
/// additional work: ignoring such a statement let one REST call hide
/// the endpoint-bearing accessor window while its call-free data
/// handling remained inside the range (`rename_needs_an_anchor`). And a
/// *block* of call-free assertions is shared verification logic the
/// members genuinely duplicate, not payload, so only the lone one is
/// idiom ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
fn covered_statements_admissible(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let mut statements = Vec::new();
    collect_covered_statements(tree.root_node(), snippet.range, &mut statements);
    if statements.is_empty() {
        return false;
    }
    let kinds = call_kinds(snippet.language);
    let (with_call, without_call): (Vec<&Node<'_>>, Vec<&Node<'_>>) = statements
        .iter()
        .partition(|node| subtree_contains_call(**node, kinds));
    match without_call.as_slice() {
        [] => true,
        [lone] => asserts::is_assert_on_call_bound_value(**lone, &with_call, snippet),
        _ => false,
    }
}

/// Collects the outermost complete statement-shaped nodes inside `range`.
fn collect_covered_statements<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    out: &mut Vec<Node<'tree>>,
) {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }
    if node.start_byte() >= range.start
        && node.end_byte() <= range.end
        && is_statement_shape(node.kind())
    {
        out.push(node);
        return;
    }
    for child in named_children(node) {
        collect_covered_statements(child, range, out);
    }
}

/// Statement and binding declarations used by the grammars this filter scans.
fn is_statement_shape(kind: &str) -> bool {
    kind.ends_with("_statement")
        || matches!(
            kind,
            "assignment"
                | "expression_statement"
                | "lexical_declaration"
                | "local_variable_declaration"
                | "variable_declaration"
        )
}

/// Whether `node` contains a call production for its language.
fn subtree_contains_call(node: Node<'_>, kinds: &[&str]) -> bool {
    if kinds.contains(&node.kind()) {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| subtree_contains_call(child, kinds))
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
    for child in named_children(node) {
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
    for child in named_children(args) {
        collect_call_shapes(child, walk, out);
    }
}

/// Compares call sequence shape, ignoring literal payloads.
fn same_call_headers(calls: &[CallShape], expected: &[CallShape]) -> bool {
    calls.len() == expected.len()
        && calls.iter().zip(expected).all(|(call, base)| {
            call.callee == base.callee && call.arity == base.arity && call.keywords == base.keywords
        })
}

/// Returns true when `index` has intentional literal variation across
/// all call sequences.
fn sequence_position_differs(sequences: &[&[CallShape]], index: usize) -> bool {
    let calls: Vec<&CallShape> = sequences
        .iter()
        .filter_map(|sequence| sequence.get(index))
        .collect();
    calls.len() == sequences.len() && has_differing_string_literals(calls)
}

/// Returns the set of tree-sitter node kinds that count as call
/// expressions per language.
const fn call_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["call"],
        b"csharp" => &["invocation_expression"],
        b"rust" => &["call_expression", "macro_invocation"],
        // Dart and ECMAScript both name their call node `call_expression`
        // and expose the same `function` / `arguments` fields the other
        // languages do, so they share one arm — for two separate reasons
        // worth keeping written down.
        //
        // Dart: `call_expression` exposes a `function` field; the
        // `constructor_invocation` node does not, so it is intentionally
        // excluded from literal-variation comparison.
        //
        // gh #284/#285: ECMAScript was absent from this map entirely, so
        // the filter could not fire for **any** JavaScript/TypeScript
        // cluster however plainly it was literal-variation scaffolding —
        // seven independent codec diagnostics sharing one
        // `expectErrorMessages` helper rendered `nearly_identical` at
        // `fused 0.86`, and a run of `expect(x).toContain("…")` lines
        // rendered `identical` at `1.00`.
        b"dart" | b"javascript" | b"typescript" | b"tsx" => &["call_expression"],
        _ => &[],
    }
}

/// Returns true when at least one positional argument index has
/// differing string-literal bytes across the cluster. Non-string
/// arguments are ignored — the heuristic only fires when the
/// distinguishing variation is in literal text.
fn has_differing_string_literals<'c>(calls: impl IntoIterator<Item = &'c CallShape>) -> bool {
    let calls: Vec<&CallShape> = calls.into_iter().collect();
    let Some(first) = calls.first() else {
        return false;
    };
    let agreements: Vec<LiteralAgreement> = (0..first.arguments.len())
        .map(|index| literal_agreement(&calls, index))
        .collect();
    !agreements.contains(&LiteralAgreement::Incomparable)
        && agreements.contains(&LiteralAgreement::Differs)
}

/// How one positional argument index reads across the cluster.
#[derive(PartialEq, Eq)]
enum LiteralAgreement {
    /// The first member holds no string literal at this index, so the
    /// position says nothing about literal variation either way.
    NotAString,
    /// Every member holds the same string-literal bytes here.
    Same,
    /// Some member holds different string-literal bytes here — the
    /// intentional test-data variation the filter looks for.
    Differs,
    /// Some member holds a non-string where the first holds a string,
    /// so the calls are not comparable as literal variation at all.
    Incomparable,
}

/// Compares argument `index` of every member against the first member.
fn literal_agreement(calls: &[&CallShape], index: usize) -> LiteralAgreement {
    let Some(Some(ArgShape::StringLiteral(baseline))) =
        calls.first().map(|call| call.arguments.get(index))
    else {
        return LiteralAgreement::NotAString;
    };
    let mut agreement = LiteralAgreement::Same;
    for call in calls.iter().skip(1) {
        match call.arguments.get(index) {
            Some(ArgShape::StringLiteral(bytes)) if bytes != baseline => {
                agreement = LiteralAgreement::Differs;
            }
            Some(ArgShape::StringLiteral(_)) => {}
            _ => return LiteralAgreement::Incomparable,
        }
    }
    agreement
}
