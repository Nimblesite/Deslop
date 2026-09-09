//! The covered-statement precondition of the literal-variation filter
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]), and the
//! statement shapes every rule of the filter reads.

use tree_sitter::Node;

use super::{asserts, call_kinds, Snippet};
use crate::{
    ast::named_children,
    cluster_filters::{node_search::KindSearch, parse_for},
};

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
pub(super) fn covered_statements_admissible(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let statements =
        KindSearch::enclosed(snippet.range, is_statement_shape).nodes(tree.root_node());
    let kinds = call_kinds(snippet.language);
    let (with_call, without_call): (Vec<&Node<'_>>, Vec<&Node<'_>>) = statements
        .iter()
        .partition(|node| subtree_contains_call(**node, kinds));
    !statements.is_empty() && call_free_admissible(&without_call, &with_call, &statements, snippet)
}

/// Which call-free statements the covered set may carry: none, the lone
/// assertion on a call-bound value, or that assertion preceded by the
/// literal tautology it reads
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]).
/// Three or more never qualify.
fn call_free_admissible(
    without_call: &[&Node<'_>],
    with_call: &[&Node<'_>],
    covered: &[Node<'_>],
    snippet: &Snippet<'_>,
) -> bool {
    match without_call {
        [] => true,
        [lone] => asserts::is_assert_on_call_bound_value(**lone, with_call, snippet),
        [tautology, assertion] => {
            asserts::is_literal_tautology_pair([tautology, assertion], with_call, covered, snippet)
        }
        // A whole scenario *run*: the widest-window selection
        // ([PIPELINE-RANK-WORST-FIRST]) may sweep several scenario cells
        // into one member, and each cell carries its own trailing
        // acceptance assert. Every call-free statement must be an
        // assertion on a value bound by the covered call that precedes
        // it, and the preceding calls must differ — one call with a run
        // of shared asserts is shared verification logic the members
        // genuinely duplicate, not the per-cell acceptance of the test
        // idiom ([CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
        _ => scenario_run_acceptance(covered, with_call, snippet),
    }
}

/// True when every call-free statement is an assertion on a value bound
/// by the covered call immediately preceding it, and those preceding
/// calls are not all the same call.
fn scenario_run_acceptance(
    covered: &[Node<'_>],
    with_call: &[&Node<'_>],
    snippet: &Snippet<'_>,
) -> bool {
    let kinds = call_kinds(snippet.language);
    let mut preceding_calls: Vec<usize> = Vec::new();
    let mut last_call_start = None;
    for statement in covered {
        if subtree_contains_call(*statement, kinds) {
            last_call_start = Some(statement.start_byte());
            continue;
        }
        if !asserts::is_assert_on_call_bound_value(*statement, with_call, snippet) {
            return false;
        }
        if let Some(start) = last_call_start {
            preceding_calls.push(start);
        }
    }
    // Every assert must sit behind a covered call, and the calls must
    // differ: per-cell acceptance, never a shared verification block.
    preceding_calls.len() == count_call_free(covered, kinds)
        && preceding_calls
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            >= 2
}

/// The number of covered statements carrying no call production.
fn count_call_free(covered: &[Node<'_>], kinds: &[&str]) -> usize {
    covered
        .iter()
        .filter(|statement| !subtree_contains_call(**statement, kinds))
        .count()
}

/// Quarantined statement classifier for [CLONE-NOISE-LITERAL-VARIATION-CALLS].
// The deleted suffix/declaration allowlist omitted Rust `let_declaration`.
// A registry-and-registration run therefore had no covered statements and
// escaped scaffolding suppression. Pinned by the failing CLI test
// `registry_call_payload_variation_keeps_only_authored_control`.
// AGENTS.md mandates this panic and its specific lint exception.
#[allow(clippy::panic, reason = "mandated accuracy quarantine")]
pub(super) fn is_statement_shape(_kind: &str) -> bool {
    panic!("accuracy quarantine: Rust call-scaffolding statements were not classified")
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
