//! [FUSED-CONTENT-GATE-AUTHORED-RUN] A method is not two adjacent methods.

use std::hash::BuildHasher;

use crate::{cluster::scope::DeclarationScopes, fingerprint::Fingerprint, pair::CandidatePair};

/// One scan's AST scope facts, computed once per fingerprint.
pub(super) struct AuthoredScopeMemo {
    /// Whether the endpoint exactly names one whole function declaration.
    single: Vec<Option<bool>>,
    /// Whether a non-single endpoint opens and closes on whole functions.
    run: Vec<Option<bool>>,
}

impl AuthoredScopeMemo {
    /// Reserves one cheap role slot per fingerprint in the candidate corpus.
    pub(super) fn new(count: usize) -> Self {
        Self {
            single: vec![None; count],
            run: vec![None; count],
        }
    }

    /// True when an edge mistakes one method for a run of several methods.
    pub(super) fn one_to_many<L: BuildHasher>(
        &mut self,
        pair: &CandidatePair,
        left: &Fingerprint,
        right: &Fingerprint,
        scopes: &DeclarationScopes<'_, L>,
    ) -> bool {
        if left.hash == right.hash {
            return false;
        }
        let left_single = cached(&mut self.single, pair.left, || {
            scopes.aligned_function(left).is_some()
        });
        let right_single = cached(&mut self.single, pair.right, || {
            scopes.aligned_function(right).is_some()
        });
        let other = match (left_single, right_single) {
            (true, false) => Some((pair.right, right)),
            (false, true) => Some((pair.left, left)),
            _ => None,
        };
        other.is_some_and(|(index, member)| {
            cached(&mut self.run, index, || scopes.aligned_function_run(member))
        })
    }
}

/// Memoizes one AST fact by fingerprint index; malformed indices cannot admit a pair.
fn cached(slots: &mut [Option<bool>], index: usize, calculate: impl FnOnce() -> bool) -> bool {
    slots
        .get_mut(index)
        .is_some_and(|slot| *slot.get_or_insert_with(calculate))
}
