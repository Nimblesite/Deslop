//! Ordered-call scenario scaffolding classification.

use std::sync::Arc;

use super::{
    call_sequence, pair_is_copy_paste, same_call_headers, sequence_position_differs, CallShape,
};
use crate::cluster_filters::{snippets::CallSequence, ParseCache, Snippet};

/// Detects body-range clusters whose contained call sequence has the
/// same callees but intentionally different literal test data.
///
/// Every position must vary, except an invariant no-literal adapter whose
/// bound result flows into a later varying call. Such an adapter is connective
/// scenario plumbing, not an independent reusable assertion. An invariant
/// literal-bearing call always blocks suppression, as does any invariant call
/// whose result is not consumed by the varying payload.
pub(super) fn is_literal_variation_call_sequence(
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> bool {
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
    sequences.is_some_and(|sequences| sequence_is_scenario_scaffolding(&sequences))
}

/// True when the members share one ordered call header and every invariant
/// position is only a bound adapter into a later varying position.
fn sequence_is_scenario_scaffolding(sequences: &[&[CallShape]]) -> bool {
    let Some(first) = sequences.first() else {
        return false;
    };
    let shared_len = sequences.iter().map(|seq| seq.len()).min().unwrap_or(0);
    if shared_len == 0 {
        return false;
    }
    // Every member must carry the same ordered call header as a prefix.
    // The overlap collapse selects the *widest* window per run
    // ([PIPELINE-RANK-WORST-FIRST]), so one occurrence may sweep several
    // scenario members: the shared skeleton is the shortest sequence, and
    // the longer members are more of the same scenario, never a reason to
    // decline the suppression the skeleton describes.
    let Some(first_header) = first.get(..shared_len) else {
        return false;
    };
    for sequence in sequences {
        let Some(sequence_header) = sequence.get(..shared_len) else {
            return false;
        };
        if !same_call_headers(sequence_header, first_header) {
            return false;
        }
    }
    let varying: Vec<bool> = (0..shared_len)
        .map(|index| sequence_position_differs(sequences, index))
        .collect();
    varying.contains(&true)
        && varying.iter().enumerate().all(|(index, differs)| {
            *differs
                || position_carries_body(sequences, index)
                || invariant_position_flows_to_variation(sequences, &varying, index)
        })
        && !sequence_pair_is_copy_paste(sequences)
}

/// Whether the call at `index` carries a body in any member. Such a
/// wrapper — a test case, a `describe` block — is neutral in the
/// sequence: the calls of its body are the positions that decide, and
/// its own header literal is never payload
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
fn position_carries_body(sequences: &[&[CallShape]], index: usize) -> bool {
    sequences
        .iter()
        .any(|sequence| sequence.get(index).is_some_and(CallShape::carries_body))
}

/// Whether one invariant no-literal call only adapts a bound value for a
/// later literal-varying call in every member.
fn invariant_position_flows_to_variation(
    sequences: &[&[CallShape]],
    varying: &[bool],
    index: usize,
) -> bool {
    sequences.iter().all(|sequence| {
        let Some(call) = sequence.get(index) else {
            return false;
        };
        !call.carries_string_literal()
            && call.result_binding.as_ref().is_some_and(|binding| {
                sequence
                    .iter()
                    .enumerate()
                    .skip(index.saturating_add(1))
                    .any(|(later, consumer)| {
                        varying.get(later).copied().unwrap_or(false)
                            && consumer.consumed_identifiers.contains(binding)
                    })
            })
    })
}

/// The sequence form of [`pair_is_copy_paste`]: same position, same
/// callee, two members, differing interpolated literal.
fn sequence_pair_is_copy_paste(sequences: &[&[CallShape]]) -> bool {
    sequences.len() == 2
        && sequences.first().is_some_and(|first| {
            (0..first.len()).any(|index| {
                let position: Vec<&CallShape> =
                    sequences.iter().filter_map(|seq| seq.get(index)).collect();
                pair_is_copy_paste(&position)
            })
        })
}
