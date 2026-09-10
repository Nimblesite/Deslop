//! Call-expression shape analysis shared by the language-agnostic
//! literal-variation cluster filter
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).

use std::sync::Arc;

use callee::{call_shape_from_node, CalleePart};
use tree_sitter::Node;

use super::{enclosing_kind, node_search::KindSearch, parse_for, ParseCache, Snippet};
use crate::ast::{named_children, ByteRange};

/// Per-argument shape extraction for the filter.
mod args;

/// Canonical callee headers and nested receiver-call argument shapes.
mod callee;

/// Assertion admission for the covered-statement rule.
mod asserts;

/// The covered-statement precondition and the statement shapes it reads.
mod statements;
use statements::{covered_statements_admissible, is_statement_shape};

/// Bound-result flow for invariant adapter calls in scenario scaffolding.
mod dataflow;

/// Ordered-call scenario scaffolding classification.
mod sequence;

/// Detects literal-variation call scaffolding
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]): a cluster whose members all
/// resolve to the same callee/arity call shape — one enclosing call, or
/// the same ordered call sequence — with **at least one string literal
/// argument differing** across members.
///
/// Two-member families are the render pass's to judge (gh #467,
/// gh #478): the split pass defers them — an early conviction there
/// never reaches render, and the pair's publish-or-suppress verdict is
/// [`pair_is_copy_paste`]'s content call.
pub(super) fn is_literal_variation_call_cluster(
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> bool {
    let calls: Option<Vec<Arc<CallShape>>> = snippets
        .iter()
        .map(|snippet| cache.call_shape(snippet, || call_shape(snippet)))
        .collect();
    is_literal_variation_call_set(calls)
        || sequence::is_literal_variation_call_sequence(snippets, cache)
}

/// Applies the literal-variation rule to one comparable call per
/// cluster member.
fn is_literal_variation_call_set(calls: Option<Vec<Arc<CallShape>>>) -> bool {
    let Some(calls) = calls else { return false };
    let Some(first) = calls.first() else {
        return false;
    };
    // A call carrying a body is judged by that body's own calls, which
    // the sequence rule reads; its header literal proves nothing.
    if calls.iter().any(|call| call.carries_body()) {
        return false;
    }
    if !calls.iter().all(|call| call.callee == first.callee) {
        return false;
    }
    if !calls.iter().all(|call| call.arity == first.arity) {
        return false;
    }
    if !calls.iter().all(|call| call.keywords == first.keywords) {
        return false;
    }
    let members: Vec<&CallShape> = calls.iter().map(Arc::as_ref).collect();
    has_differing_string_literals(members.iter().copied()) && !pair_is_copy_paste(&members)
}

/// Distilled view of a call expression used to compare cluster members.
#[derive(Clone)]
pub(crate) struct CallShape {
    /// AST callee shape preserving called names and member selectors,
    /// with receiver names and nested string payloads normalised.
    callee: Vec<CalleePart>,
    /// Number of argument slots, including nested callee invocations.
    arity: usize,
    /// Keyword each argument is passed under, positionally, `None` for
    /// positional arguments. Part of the header: two calls naming
    /// different parameters are different shapes ([`keyword_name`]).
    keywords: Vec<Option<Vec<u8>>>,
    /// Per-argument summary used for literal-variation detection.
    arguments: Vec<ArgShape>,
    /// Local name this call's result is assigned to, when any.
    result_binding: Option<Vec<u8>>,
    /// Raw identifiers this call consumes, through its arguments or
    /// through an invocation spelled inside its callee.
    consumed_identifiers: Vec<Vec<u8>>,
}

impl CallShape {
    /// Whether any argument carries statements, making the call a body
    /// holder rather than a literal-varying scaffold
    /// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
    pub(super) fn carries_body(&self) -> bool {
        self.arguments
            .iter()
            .any(|argument| matches!(argument, ArgShape::Body))
    }

    /// Whether any argument is authored string payload.
    pub(super) fn carries_string_literal(&self) -> bool {
        self.arguments.iter().any(|argument| {
            matches!(
                argument,
                ArgShape::StringLiteral(_, _) | ArgShape::LiteralWrapper(_, _)
            )
        })
    }
}

/// Per-argument summary recorded for each call.
#[derive(Clone)]
enum ArgShape {
    /// Raw bytes of a string-literal argument (or string content of an
    /// f-string / interpolated string), and whether the literal embeds
    /// an interpolation — the authored-code signal of gh #467.
    StringLiteral(Vec<u8>, bool),
    /// An argument carrying statements — a test body, a callback block.
    /// Statements are authored logic the members duplicate, never
    /// payload handed to a shared callee, so a call carrying one is
    /// judged by that body and its literals prove nothing
    /// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
    Body,
    /// A call wrapping only literal payload — `PathBuf::from("x")`. Held
    /// as two separate facts, because they answer different questions:
    /// the literal is the data a family varies, and the callee is *which
    /// code runs*. Folded into one byte string, `Url.parse("x")` against
    /// `Uri.resolve("x")` reads as a differing literal and the pair is
    /// suppressed as scaffolding — a false negative on a real
    /// behavioural difference.
    ///
    /// The callee is the *canonical* header, not raw bytes: receiver
    /// names are normalised away exactly as they are everywhere else, so
    /// `nav.locator("a")` and `page.locator("b")` remain one shape
    /// varying its payload, while `parse` against `resolve` does not.
    LiteralWrapper(Vec<CalleePart>, Vec<u8>),
    /// Anything else — non-string literal, identifier, sub-expression.
    Other,
}

/// Extracts the [`CallShape`] for the call `snippet` is
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-MEMBER-CALL]). Returns `None`
/// when no call is present.
fn call_shape(snippet: &Snippet<'_>) -> Option<CallShape> {
    let tree = parse_for(snippet)?;
    let kinds = call_kinds(snippet.language);
    let enclosing = enclosing_kind(tree.root_node(), snippet.range, kinds)
        .and_then(|call| call_shape_from_node(call, snippet.source, snippet.language));
    if let Some(shape) = enclosing.as_ref().filter(|shape| !shape.carries_body()) {
        return Some(shape.clone());
    }
    // Either no call encloses the member — an awaited check in a plain
    // function body — or the enclosing call only carries the member
    // inside a body, a test case around that check, and a body is never
    // judged by the name beside it. A member that is an expression around
    // one call is judged by that call; a member holding a complete
    // statement is a run, which is the sequence rule's question, and a
    // run is never judged by the one call it happens to contain
    // ([CLONE-NOISE-LITERAL-VARIATION-CALLS-MEMBER-CALL]).
    held_call(tree.root_node(), snippet, kinds)
        .and_then(|call| call_shape_from_node(call, snippet.source, snippet.language))
        .or(enclosing)
}

/// The one outermost call `snippet` holds, when `snippet` is an
/// expression — it holds no complete statement — and holds exactly one.
fn held_call<'tree>(
    root: Node<'tree>,
    snippet: &Snippet<'_>,
    kinds: &[&str],
) -> Option<Node<'tree>> {
    let statements = KindSearch::enclosed(snippet.range, is_statement_shape).nodes(root);
    if !statements.is_empty() {
        return None;
    }
    KindSearch::enclosed(snippet.range, |kind| kinds.contains(&kind)).sole_node(root)
}

/// Computes the fused literal-variation sequence cell for one snippet:
/// the covered-statement flag and the in-range call sequence, both pure
/// functions of `(file, range)` and memoised together
/// ([PERF-FLUTTER-TODO-CORPUS]).
fn call_sequence(snippet: &Snippet<'_>) -> super::snippets::CallSequence {
    let shapes = call_shapes_in_range(snippet);
    let admissible = covered_statements_admissible(snippet);
    super::snippets::CallSequence {
        statements_admissible: admissible,
        shapes,
    }
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
/// receiver's call names and payloads already belong to the enclosing
/// [`CallShape`], so the information is counted once. Arguments are still
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
            same_callee_where_it_matters(call, base)
                && call.arity == base.arity
                && call.keywords == base.keywords
        })
}

/// A literal-bearing position must name one callee — "the same helper
/// called with different data" is the scaffold. A literal-free position
/// is plumbing, and its callee may differ between members
/// (`generateRust` in one scenario, `generateTypeScript` in another): it
/// still has to be the bound-and-consumed adapter the sequence rule
/// demands of every invariant position
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
fn same_callee_where_it_matters(call: &CallShape, base: &CallShape) -> bool {
    call.callee == base.callee || (!call.carries_string_literal() && !base.carries_string_literal())
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

/// Returns true when the calls vary as literal-variation scaffolding:
/// **every** literal-bearing argument position differs across the
/// cluster, and at least one does. A position that carries no string
/// literal is neutral. An invariant literal-bearing position is shared
/// authored logic and blocks suppression
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
fn has_differing_string_literals<'c>(calls: impl IntoIterator<Item = &'c CallShape>) -> bool {
    let calls: Vec<&CallShape> = calls.into_iter().collect();
    if calls.iter().any(|call| call.carries_body()) {
        return false;
    }
    let Some(first) = calls.first() else {
        return false;
    };
    let agreements: Vec<LiteralAgreement> = (0..first.arguments.len())
        .map(|index| literal_agreement(&calls, index))
        .collect();
    !agreements.contains(&LiteralAgreement::Incomparable)
        && !agreements.contains(&LiteralAgreement::Same)
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
    match calls.first().and_then(|call| call.arguments.get(index)) {
        Some(ArgShape::StringLiteral(baseline, _)) => string_agreement(calls, index, baseline),
        Some(ArgShape::LiteralWrapper(callee, baseline)) => {
            wrapper_agreement(calls, index, callee, baseline)
        }
        _ => LiteralAgreement::NotAString,
    }
}

/// Bare string payloads agree, differ, or are not comparable at all.
fn string_agreement(calls: &[&CallShape], index: usize, baseline: &[u8]) -> LiteralAgreement {
    let mut agreement = LiteralAgreement::Same;
    for call in calls.iter().skip(1) {
        match call.arguments.get(index) {
            Some(ArgShape::StringLiteral(bytes, _)) if bytes != baseline => {
                agreement = LiteralAgreement::Differs;
            }
            Some(ArgShape::StringLiteral(_, _)) => {}
            _ => return LiteralAgreement::Incomparable,
        }
    }
    agreement
}

/// A wrapped payload varies only while every member reaches for the
/// same constructor. A member that wraps its literal in a *different*
/// callee is running different code, which is a difference to report and
/// never one family's varying test data, so the position is incomparable
/// and blocks the suppression outright.
fn wrapper_agreement(
    calls: &[&CallShape],
    index: usize,
    callee: &[CalleePart],
    baseline: &[u8],
) -> LiteralAgreement {
    let mut agreement = LiteralAgreement::Same;
    for call in calls.iter().skip(1) {
        let Some(ArgShape::LiteralWrapper(other_callee, bytes)) = call.arguments.get(index) else {
            return LiteralAgreement::Incomparable;
        };
        if other_callee != callee {
            return LiteralAgreement::Incomparable;
        }
        if bytes != baseline {
            agreement = LiteralAgreement::Differs;
        }
    }
    agreement
}

/// gh #467: whether a two-member literal-variation family is a
/// copy-pasted pair rather than parameterisable scaffolding. A pair
/// whose differing string argument is an authored interpolation — an
/// f-string route, a template substitution — publishes: the variation
/// is code choosing data, no helper exists for a family to
/// parameterise over, and suppressing it deletes a visible duplicate.
/// Plain-literal pairs stay suppressed: their variation is data handed
/// to a shared callee, which is the scaffolding shape the filter names.
fn pair_is_copy_paste(calls: &[&CallShape]) -> bool {
    calls.len() == 2
        && calls.first().is_some_and(|first| {
            (0..first.arguments.len()).any(|index| {
                literal_agreement(calls, index) == LiteralAgreement::Differs
                    && calls.iter().any(|call| {
                        matches!(
                            call.arguments.get(index),
                            Some(ArgShape::StringLiteral(_, true))
                        )
                    })
            })
        })
}

#[cfg(test)]
mod tests;
