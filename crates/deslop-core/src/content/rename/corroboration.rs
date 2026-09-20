//! [TECH-PMATCH-BAKER] The modal bijection and the corroboration ledger
//! read off one pair's aligned identifier positions.

use std::collections::BTreeMap;

use super::{pair_counts, substituted_pairs, RENAME_CORROBORATION_MIN_OCCURRENCES};

/// The bidirectionally-modal substitution test shared by the substance
/// and rename measures: a position is explained when its pair is the
/// modal partner in both directions. A genuine rename maps every
/// occurrence of a name to one new name; scattergun similarity does
/// not. The caller chooses the population: [`mapping_consistency`]
/// maps over every identifier position (identity included), while
/// [`rename_mapping`] maps over the substituted pairs alone.
pub(in crate::content) struct ModalBijection {
    /// Modal partner of each left key.
    forward: BTreeMap<u64, u64>,
    /// Modal partner of each right key.
    backward: BTreeMap<u64, u64>,
}

impl ModalBijection {
    /// Builds the two modal maps over one pair's identifier positions.
    pub(in crate::content) fn over(identifiers: &[(u64, u64)]) -> Self {
        Self {
            forward: modal_partners(identifiers.iter().map(|(left, right)| (*left, *right))),
            backward: modal_partners(identifiers.iter().map(|(left, right)| (*right, *left))),
        }
    }

    /// True when the pair is the modal partner in both directions.
    pub(in crate::content) fn explains(&self, (left, right): &(u64, u64)) -> bool {
        self.forward.get(left) == Some(right) && self.backward.get(right) == Some(left)
    }
}

/// [FUSED-CONTENT-GATE-CALL-TARGET] [TECH-PMATCH-BAKER] One pair's
/// corroboration ledger: which of its substitutions are repeated
/// rename evidence rather than unconstrained wildcards.
///
/// Built once per pair and asked about as many substitutions as the
/// caller has questions — every receiver a changed call target is
/// selected on reads the same ledger.
pub(in crate::content) struct Corroboration {
    /// Occurrences of each substituted key pair.
    counts: BTreeMap<(u64, u64), usize>,
    /// Whether every substitution follows the modal bijection; one
    /// name mapped two ways corroborates nothing anywhere.
    consistent: bool,
}

impl Corroboration {
    /// Reads the ledger of one pair's aligned identifier positions.
    pub(in crate::content) fn over(identifiers: &[(u64, u64)]) -> Self {
        let substitutions = substituted_pairs(identifiers);
        let bijection = ModalBijection::over(&substitutions);
        Self {
            consistent: substitutions.iter().all(|pair| bijection.explains(pair)),
            counts: pair_counts(substitutions.into_iter()),
        }
    }

    /// A repeated, unambiguous substitution inside a contradiction-free
    /// mapping — the rename evidence [TECH-PMATCH-BAKER] recognises.
    pub(in crate::content) fn admits(&self, keys: (u64, u64)) -> bool {
        keys.0 != keys.1
            && self.consistent
            && self.counts.get(&keys).copied().unwrap_or_default()
                >= RENAME_CORROBORATION_MIN_OCCURRENCES
    }
}

/// Modal partner per key: the partner seen most often. Counting and
/// folding run over [`BTreeMap`]s in ascending order and replacement
/// requires a strictly greater count, so ties resolve to the smallest
/// partner key and the map is deterministic across runs.
fn modal_partners(pairs: impl Iterator<Item = (u64, u64)>) -> BTreeMap<u64, u64> {
    let mut modes: BTreeMap<u64, (u64, usize)> = BTreeMap::new();
    for ((key, partner), count) in pair_counts(pairs) {
        let best = modes.entry(key).or_insert((partner, count));
        if count > best.1 {
            *best = (partner, count);
        }
    }
    modes
        .into_iter()
        .map(|(key, (partner, _))| (key, partner))
        .collect()
}
