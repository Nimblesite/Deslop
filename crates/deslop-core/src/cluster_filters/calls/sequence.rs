//! Ordered-call scenario scaffolding classification.

use std::sync::Arc;

use super::{
    call_sequence, pair_is_copy_paste, same_call_headers, sequence_position_differs, CallShape,
};
use crate::cluster_filters::{snippets::CallSequence, ParseCache, Snippet};

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
    sequences.is_some_and(|sequences| every_sequence_position_varies(&sequences))
}

/// True when the members share one non-empty ordered call header and
/// every position in it carries differing string literals — except a
/// two-member pair whose differing literal is authored interpolation,
/// which publishes (gh #467).
fn every_sequence_position_varies(sequences: &[&[CallShape]]) -> bool {
    let Some(first) = sequences.first() else {
        return false;
    };
    if first.is_empty() || !sequences.iter().all(|seq| same_call_headers(seq, first)) {
        return false;
    }
    (0..first.len()).all(|index| sequence_position_differs(sequences, index))
        && !sequence_pair_is_copy_paste(sequences)
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
