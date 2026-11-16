//! [PIPELINE-INCREMENTAL-ANALYSIS-VERDICTS] Per-pair measurements carried
//! across live edits.
//!
//! The shared-subtree rescue and the content gate judge one candidate
//! pair at a time. Each verdict reads the pair's two files and one
//! corpus-wide input: the exact clones that relate to its endpoints, by
//! containment ([`crate::pair::ExactClones`]). An exact clone between two
//! unchanged files appears or vanishes only when a pair incident to one of
//! its records does, and that happens only to a record of a group the
//! edit re-paired ([PIPELINE-INCREMENTAL-ANALYSIS-PAIRS]) or to an
//! endpoint of a copy-recovery pair that came or went. So a pair whose
//! files did not change and whose endpoints overlap none of those records
//! keeps its previous verdicts; every other pair is measured again.

use std::collections::{HashMap, HashSet};

use super::PipelineSession;
use crate::{
    ast::{ByteRange, NormalizedNode},
    cluster::Folded,
    cluster_filters::ParseCache,
    fingerprint::Fingerprint,
    overlap::{apply_shared_subtree_rescue_to, RescueCorpus},
    pair::{apply_pair_content_gate, CandidatePair, FusedCluster},
    state::FileId,
};

/// One pair's rescue outcome: overlap and verified async core.
type RescueRow = (usize, usize, f64, bool);
/// One pair's content-gate verdict.
type GateRow = (usize, usize, bool);

/// Per-pair measurements one render made, in that render's layout.
#[derive(Debug, Default)]
pub(super) struct Verdicts {
    /// Rescue outcome of every candidate pair, sorted.
    rescue: Vec<RescueRow>,
    /// Gate verdict of every pair the gate judged, sorted.
    gate: Vec<GateRow>,
    /// Pairs copy recovery added, sorted.
    recovered: Vec<(usize, usize)>,
    /// Every fused cluster's fold, by side and members.
    folds: HashMap<(bool, Vec<usize>), Folded>,
}

impl Verdicts {
    /// These verdicts re-indexed by `remap`, dropping every row with an
    /// endpoint `remap` rejects.
    pub(super) fn remapped(self, remap: &dyn Fn(usize) -> Option<usize>) -> Self {
        let mut carried = Self {
            rescue: remap_rows(self.rescue, remap, |row| (&mut row.0, &mut row.1)),
            gate: remap_rows(self.gate, remap, |row| (&mut row.0, &mut row.1)),
            recovered: remap_rows(self.recovered, remap, |row| (&mut row.0, &mut row.1)),
            folds: self
                .folds
                .into_iter()
                .filter_map(|(key, folded)| remap_fold(key, folded, remap))
                .collect(),
        };
        carried.sort();
        carried
    }

    /// Puts every row in key order.
    fn sort(&mut self) {
        self.rescue.sort_unstable_by_key(|row| (row.0, row.1));
        self.gate.sort_unstable_by_key(|row| (row.0, row.1));
        self.recovered.sort_unstable();
    }
}

/// `rows` re-indexed by `remap`, dropping each row with an endpoint it rejects.
fn remap_rows<T>(
    rows: Vec<T>,
    remap: &dyn Fn(usize) -> Option<usize>,
    ends: impl Fn(&mut T) -> (&mut usize, &mut usize),
) -> Vec<T> {
    rows.into_iter()
        .filter_map(|mut row| {
            let (left, right) = ends(&mut row);
            *left = remap(*left)?;
            *right = remap(*right)?;
            Some(row)
        })
        .collect()
}

/// One fold re-indexed by `remap`, or `None` when any index is rejected.
fn remap_fold(
    (clones, members): (bool, Vec<usize>),
    folded: Folded,
    remap: &dyn Fn(usize) -> Option<usize>,
) -> Option<((bool, Vec<usize>), Folded)> {
    let members = members
        .into_iter()
        .map(remap)
        .collect::<Option<Vec<usize>>>()?;
    let folded = folded
        .into_iter()
        .map(|(group, kind)| {
            Some((
                group
                    .into_iter()
                    .map(remap)
                    .collect::<Option<Vec<usize>>>()?,
                kind,
            ))
        })
        .collect::<Option<Folded>>()?;
    Some(((clones, members), folded))
}

/// Records whose measurements an edit may have changed, per file, with
/// the files the edit changed.
struct Unsettled {
    /// Files whose records were replaced, added or removed.
    changed: HashSet<FileId>,
    /// Ranges of unsettled records per file, by start, with the furthest
    /// end reached so far.
    reach: HashMap<FileId, Vec<(usize, usize)>>,
}

impl Unsettled {
    /// The unsettled set of `records` in `changed` files' corpus.
    fn new<'a>(changed: HashSet<FileId>, records: impl Iterator<Item = &'a Fingerprint>) -> Self {
        let mut ranges: HashMap<FileId, Vec<ByteRange>> = HashMap::new();
        for record in records {
            ranges
                .entry(record.file_id)
                .or_default()
                .push(record.byte_range);
        }
        let reach = ranges
            .into_iter()
            .map(|(file, ranges)| (file, reach_of(ranges)))
            .collect();
        Self { changed, reach }
    }

    /// Whether `record`'s file is unchanged and no unsettled record of
    /// it touches `record`'s range.
    fn settles(&self, record: &Fingerprint) -> bool {
        if self.changed.contains(&record.file_id) {
            return false;
        }
        let Some(reach) = self.reach.get(&record.file_id) else {
            return true;
        };
        let range = record.byte_range;
        let before_end = reach.partition_point(|(start, _)| *start <= range.end);
        !before_end
            .checked_sub(1)
            .and_then(|last| reach.get(last))
            .is_some_and(|(_, furthest)| *furthest >= range.start)
    }
}

/// `ranges` by start, each with the furthest end reached up to it.
fn reach_of(mut ranges: Vec<ByteRange>) -> Vec<(usize, usize)> {
    ranges.sort_unstable_by_key(|range| range.start);
    let mut furthest = 0_usize;
    ranges
        .iter()
        .map(|range| {
            furthest = furthest.max(range.end);
            (range.start, furthest)
        })
        .collect()
}

/// What the previous render measured, carried into this one.
pub(super) struct CarriedVerdicts {
    /// The previous verdicts, re-indexed into this corpus.
    verdicts: Verdicts,
    /// Records the rescue may not trust.
    rescue_unsettled: Unsettled,
    /// Records the gate may not trust, once recovery has run.
    gate_unsettled: Option<Unsettled>,
    /// The changed files and the records the edit re-paired.
    changed: HashSet<FileId>,
    /// Indices of the records the edit re-paired.
    re_paired: Vec<usize>,
}

impl CarriedVerdicts {
    /// Carries `verdicts` into a corpus whose `changed` files changed and
    /// whose `re_paired` records were re-paired.
    pub(super) fn new(
        verdicts: Verdicts,
        changed: HashSet<FileId>,
        re_paired: Vec<usize>,
        fingerprints: &[Fingerprint],
    ) -> Self {
        let records = re_paired
            .iter()
            .filter_map(|index| fingerprints.get(*index));
        Self {
            verdicts,
            rescue_unsettled: Unsettled::new(changed.clone(), records),
            gate_unsettled: None,
            changed,
            re_paired,
        }
    }

    /// Whether both endpoints of `pair` are settled under `unsettled`.
    fn settled(unsettled: &Unsettled, pair: &CandidatePair, fingerprints: &[Fingerprint]) -> bool {
        [pair.left, pair.right].into_iter().all(|index| {
            fingerprints
                .get(index)
                .is_some_and(|record| unsettled.settles(record))
        })
    }

    /// The previous rescue outcome of `pair`, when it still holds.
    fn rescue(&self, pair: &CandidatePair, fingerprints: &[Fingerprint]) -> Option<(f64, bool)> {
        if !Self::settled(&self.rescue_unsettled, pair, fingerprints) {
            return None;
        }
        let rows = &self.verdicts.rescue;
        let position = rows
            .binary_search_by_key(&(pair.left, pair.right), |row| (row.0, row.1))
            .ok()?;
        rows.get(position).map(|row| (row.2, row.3))
    }

    /// The previous fold of `fused` on side `clones`, when every member
    /// is settled.
    fn fold(
        &self,
        clones: bool,
        fused: &FusedCluster,
        fingerprints: &[Fingerprint],
    ) -> Option<Folded> {
        let unsettled = self.gate_unsettled.as_ref()?;
        let settled = fused.members.iter().all(|index| {
            fingerprints
                .get(*index)
                .is_some_and(|record| unsettled.settles(record))
        });
        if !settled {
            return None;
        }
        self.verdicts
            .folds
            .get(&(clones, fused.members.clone()))
            .cloned()
    }

    /// The previous gate verdict of `pair`, when it still holds.
    fn gate(&self, pair: &CandidatePair, fingerprints: &[Fingerprint]) -> Option<bool> {
        let unsettled = self.gate_unsettled.as_ref()?;
        if !Self::settled(unsettled, pair, fingerprints) {
            return None;
        }
        let rows = &self.verdicts.gate;
        let position = rows
            .binary_search_by_key(&(pair.left, pair.right), |row| (row.0, row.1))
            .ok()?;
        rows.get(position).map(|row| row.2)
    }

    /// `pairs` split into those the rescue must measure and those whose
    /// previous outcome still holds, applied.
    fn split_settled(
        &self,
        pairs: Vec<CandidatePair>,
        fingerprints: &[Fingerprint],
    ) -> (Vec<CandidatePair>, Vec<CandidatePair>) {
        let mut fresh = Vec::with_capacity(pairs.len());
        let mut settled = Vec::new();
        for mut pair in pairs {
            match self.rescue(&pair, fingerprints) {
                Some((overlap, verified)) => {
                    pair.shared_subtree_overlap = overlap;
                    pair.verified_async_core = verified;
                    settled.push(pair);
                }
                None => fresh.push(pair),
            }
        }
        (fresh, settled)
    }

    /// Settles the gate's unsettled set once recovery has added
    /// `recovered`: the endpoints of every recovered pair that came or
    /// went join the re-paired records.
    fn recovery_ran(&mut self, recovered: &[(usize, usize)], fingerprints: &[Fingerprint]) {
        let before: HashSet<&(usize, usize)> = self.verdicts.recovered.iter().collect();
        let now: HashSet<&(usize, usize)> = recovered.iter().collect();
        let moved = before
            .symmetric_difference(&now)
            .flat_map(|(left, right)| [*left, *right]);
        let records = self
            .re_paired
            .iter()
            .copied()
            .chain(moved)
            .filter_map(|index| fingerprints.get(index));
        self.gate_unsettled = Some(Unsettled::new(self.changed.clone(), records));
    }
}

/// One render's verdict bookkeeping: what it may reuse, and what it
/// records for the next pass.
pub(super) struct VerdictPass {
    /// The previous render's verdicts, when this pass may reuse them.
    carried: Option<CarriedVerdicts>,
    /// This render's verdicts, when a ledger will keep them.
    record: Option<Verdicts>,
}

impl VerdictPass {
    /// A pass reusing `carried` and recording when `keeps` is set.
    pub(super) fn new(carried: Option<CarriedVerdicts>, keeps: bool) -> Self {
        Self {
            carried,
            record: keeps.then(Verdicts::default),
        }
    }

    /// Applies every still-valid rescue outcome to `pairs` and moves the
    /// pairs that must be measured to the front. Returns their count.
    pub(super) fn settle_rescue(
        &self,
        pairs: &mut Vec<CandidatePair>,
        fingerprints: &[Fingerprint],
    ) -> usize {
        let Some(carried) = self.carried.as_ref() else {
            return pairs.len();
        };
        let (mut fresh, settled) = carried.split_settled(std::mem::take(pairs), fingerprints);
        let measured = fresh.len();
        fresh.extend(settled);
        *pairs = fresh;
        measured
    }

    /// Records every pair's rescue outcome.
    pub(super) fn rescued(&mut self, pairs: &[CandidatePair]) {
        if let Some(record) = self.record.as_mut() {
            record.rescue = pairs
                .iter()
                .map(|pair| {
                    let outcome = (pair.shared_subtree_overlap, pair.verified_async_core);
                    (pair.left, pair.right, outcome.0, outcome.1)
                })
                .collect();
        }
    }

    /// Records the pairs copy recovery added and settles the gate's
    /// reuse against them.
    pub(super) fn recovered(&mut self, added: &[CandidatePair], fingerprints: &[Fingerprint]) {
        let keys: Vec<(usize, usize)> = added.iter().map(|pair| (pair.left, pair.right)).collect();
        if let Some(carried) = self.carried.as_mut() {
            carried.recovery_ran(&keys, fingerprints);
        }
        if let Some(record) = self.record.as_mut() {
            record.recovered = keys;
        }
    }

    /// The previous gate verdict of `pair`, when it still holds.
    pub(super) fn gate(&self, pair: &CandidatePair, fingerprints: &[Fingerprint]) -> Option<bool> {
        self.carried.as_ref()?.gate(pair, fingerprints)
    }

    /// [PIPELINE-INCREMENTAL-ANALYSIS-CLUSTERS] The previous fold of
    /// `fused` on side `clones`, when it still holds.
    pub(super) fn fold(
        &self,
        clones: bool,
        fused: &FusedCluster,
        fingerprints: &[Fingerprint],
    ) -> Option<Folded> {
        self.carried.as_ref()?.fold(clones, fused, fingerprints)
    }

    /// Records the fold of every fused cluster on side `clones`.
    pub(super) fn folded(&mut self, clones: bool, fused: &[FusedCluster], folds: Vec<Folded>) {
        if let Some(record) = self.record.as_mut() {
            let rows = fused
                .iter()
                .zip(folds)
                .map(|(fused, folded)| ((clones, fused.members.clone()), folded));
            record.folds.extend(rows);
        }
    }

    /// Records the gate's verdict on every pair it was handed.
    fn judged(&mut self, handed: &[(usize, usize)], admitted: &[CandidatePair]) {
        let Some(record) = self.record.as_mut() else {
            return;
        };
        let kept: HashSet<(usize, usize)> = admitted
            .iter()
            .map(|pair| (pair.left, pair.right))
            .collect();
        record.gate = handed
            .iter()
            .map(|&(left, right)| (left, right, kept.contains(&(left, right))))
            .collect();
    }

    /// This render's verdicts, in key order, for the ledger.
    pub(super) fn finish(self) -> Option<Verdicts> {
        self.record.map(|mut verdicts| {
            verdicts.sort();
            verdicts
        })
    }
}

impl PipelineSession {
    /// [PIPELINE-INCREMENTAL-ANALYSIS-VERDICTS] The shared-subtree rescue
    /// over `pairs`, measuring only the pairs `pass` cannot vouch for.
    /// Returns how many it measured.
    pub(super) fn rescue_carrying(
        &self,
        pairs: &mut Vec<CandidatePair>,
        trees: &[NormalizedNode],
        pass: &mut VerdictPass,
    ) -> usize {
        let fingerprints = self.store.fingerprints();
        let measured = pass.settle_rescue(pairs, fingerprints);
        let corpus = RescueCorpus {
            fingerprints,
            trees,
            sources: &self.sources,
            languages: &self.file_languages,
            min_nodes: usize::try_from(self.min_nodes).unwrap_or(usize::MAX),
        };
        apply_shared_subtree_rescue_to(pairs, measured, &corpus);
        pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
        pass.rescued(pairs);
        measured
    }

    /// [PIPELINE-INCREMENTAL-ANALYSIS-VERDICTS] The content gate over
    /// `pairs`, judging only the pairs `pass` cannot vouch for. Returns
    /// how many it judged.
    pub(super) fn gate_carrying(
        &self,
        pairs: &mut Vec<CandidatePair>,
        trees: &[NormalizedNode],
        parse_cache: &ParseCache,
        pass: &mut VerdictPass,
    ) -> usize {
        let fingerprints = self.store.fingerprints();
        let handed: Vec<(usize, usize)> =
            pairs.iter().map(|pair| (pair.left, pair.right)).collect();
        let carried = |pair: &CandidatePair| pass.gate(pair, fingerprints);
        let judged = pairs.iter().filter(|pair| carried(pair).is_none()).count();
        let (sources, languages) = (&self.sources, &self.file_languages);
        apply_pair_content_gate(
            pairs,
            fingerprints,
            trees,
            sources,
            languages,
            parse_cache,
            &carried,
        );
        pass.judged(&handed, pairs);
        judged
    }
}
