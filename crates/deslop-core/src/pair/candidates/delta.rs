//! [PIPELINE-INCREMENTAL-ANALYSIS-PAIRS] Re-pairing only what an edit
//! touched ([LIVE-PERF-BUDGETS]).
//!
//! The kept pair set is a function of bucket memberships alone. Each
//! structural (Merkle) bucket and each LSH band group emits its own star
//! ([`bucket_star`], [`emit_sorted_star`]); construction and the gate are
//! pure in the two endpoints ([`PairBuilder::admit`]); the batch build
//! sorts what survives. A pair is kept while any source still emits it,
//! which its source bits record.
//!
//! An edit changes only the buckets that held the changed files' old or
//! new records. Every other bucket keeps its members, so its star and the
//! bits it set are unchanged. A dirty bucket is re-starred in two steps:
//! the star its unchanged members alone would draw loses the bucket's
//! bit, then the bucket's full star sets it. Unchanged members keep their
//! relative order, so whenever the old star's centre was unchanged it is
//! the smallest unchanged member, and its pairs among unchanged members
//! are exactly the first step's; a centre in a changed file took its
//! pairs with it. Pairs touching a changed file never arrive here.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::BuildHasher,
};

use super::{
    builder::{band_source, bucket_star, PairBuilder, STRUCTURAL_SOURCE},
    order, CandidatePair,
};
use crate::{
    fingerprint::Fingerprint,
    lsh::{band_key, emit_sorted_star, Signature, SignatureLookup, BANDS, ROWS_PER_BAND},
    state::FileId,
};

/// A kept pair as `(left, right, source bits)`, `left < right`.
pub(crate) type SourcedKey = (usize, usize, u64);

/// A bucket's pairing rule: draws `members`' star into `emit`.
type StarRule<'a> = &'a dyn Fn(&[usize], &mut dyn FnMut(usize, usize));

/// The live session's kept pairs, each with the bits of the sources that
/// emitted it, positionally 1:1.
#[derive(Debug, Default)]
pub(crate) struct SourcedPairs {
    /// The kept candidates in `(left, right)` order.
    pub(crate) pairs: Vec<CandidatePair>,
    /// Source bits per candidate.
    pub(crate) sources: Vec<u64>,
}

/// The structural hashes and band keys of records an edit removed: the
/// buckets they left are dirty even when no new record joins them.
#[derive(Debug, Default)]
pub(crate) struct RetiredKeys {
    /// Structural hashes of removed records.
    hashes: HashSet<[u8; 32]>,
    /// Band keys of removed records, per band.
    bands: Vec<HashSet<[u8; 32]>>,
}

impl RetiredKeys {
    /// Records one removed record's keys.
    pub(crate) fn retire(&mut self, fingerprint: &Fingerprint, signature: Option<&Signature>) {
        let _new = self.hashes.insert(fingerprint.hash);
        let Some(signature) = signature else {
            return;
        };
        self.bands.resize_with(BANDS, HashSet::new);
        for (band, keys) in self.bands.iter_mut().enumerate() {
            let _new = keys.insert(band_key(signature, band));
        }
    }

    /// The removed records' keys in `band`.
    fn band(&self, band: usize) -> impl Iterator<Item = &[u8; 32]> {
        self.bands.get(band).into_iter().flatten()
    }
}

/// One live pass's corpus after an edit, and what the edit changed.
pub(crate) struct EditedCorpus<'a, S: BuildHasher> {
    /// Every live fingerprint, in path order.
    pub(crate) fingerprints: &'a [Fingerprint],
    /// One signature per fingerprint.
    pub(crate) signatures: &'a dyn SignatureLookup,
    /// Language per file.
    pub(crate) file_languages: &'a HashMap<FileId, &'static str, S>,
    /// Files whose records were replaced, added or removed.
    pub(crate) changed: &'a HashSet<FileId>,
    /// Keys of the records the edit removed.
    pub(crate) retired: &'a RetiredKeys,
}

impl<S: BuildHasher> EditedCorpus<'_, S> {
    /// The members of `group` outside every changed file.
    fn unchanged(&self, group: &[usize]) -> Vec<usize> {
        group
            .iter()
            .copied()
            .filter(|index| !self.is_changed(*index))
            .collect()
    }

    /// Whether record `index` belongs to a changed file.
    fn is_changed(&self, index: usize) -> bool {
        self.fingerprints
            .get(index)
            .is_some_and(|fingerprint| self.changed.contains(&fingerprint.file_id))
    }
}

/// [PIPELINE-INCREMENTAL-ANALYSIS-PAIRS] The kept pairs after an edit, from `carried` —
/// the previous kept pairs re-indexed into this corpus, sorted, none
/// touching a changed file — and the dirty buckets alone. Returns the
/// pairs and the records that were re-paired, in index order.
pub(crate) fn candidate_pairs_after_edit<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    carried: Vec<SourcedKey>,
) -> (SourcedPairs, Vec<usize>) {
    let builder = PairBuilder::new(
        corpus.fingerprints,
        corpus.signatures,
        Some(corpus.file_languages),
        false,
    );
    let changed: Vec<usize> = (0..corpus.fingerprints.len())
        .filter(|index| corpus.is_changed(*index))
        .collect();
    let mut book = SourceBook::new(carried);
    restar_structural(corpus, &builder, &changed, &mut book);
    let structural_scanned = book.scanned;
    restar_bands(corpus, &builder, &changed, &mut book);
    book.finish(&builder, structural_scanned)
}

/// Re-stars every structural bucket a changed record left or joined.
fn restar_structural<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    builder: &PairBuilder<'_, S>,
    changed: &[usize],
    book: &mut SourceBook,
) {
    let rule = |members: &[usize], emit: &mut dyn FnMut(usize, usize)| {
        bucket_star(members, corpus.fingerprints, emit);
    };
    for run in structural_groups(corpus, changed) {
        for members in builder.language_groups(&run) {
            book.restar(corpus, builder, STRUCTURAL_SOURCE, &members, &rule);
        }
    }
}

/// Re-stars every band group a changed record left or joined.
fn restar_bands<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    builder: &PairBuilder<'_, S>,
    changed: &[usize],
    book: &mut SourceBook,
) {
    let rule = |members: &[usize], emit: &mut dyn FnMut(usize, usize)| {
        emit_sorted_star(members, emit);
    };
    for band in 0..BANDS {
        for group in band_groups(corpus, band, changed) {
            book.restar(corpus, builder, band_source(band), &group, &rule);
        }
    }
}

/// The members of every structural bucket a changed record left or
/// joined, each in index order.
fn structural_groups<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    changed: &[usize],
) -> Vec<Vec<usize>> {
    let mut dirty = corpus.retired.hashes.clone();
    dirty.extend(
        changed
            .iter()
            .filter_map(|index| corpus.fingerprints.get(*index))
            .map(|fingerprint| fingerprint.hash),
    );
    let mut buckets: BTreeMap<[u8; 32], Vec<usize>> = BTreeMap::new();
    for (index, fingerprint) in corpus.fingerprints.iter().enumerate() {
        if dirty.contains(&fingerprint.hash) {
            buckets.entry(fingerprint.hash).or_default().push(index);
        }
    }
    buckets.into_values().collect()
}

/// The members of every `band` group a changed record left or joined,
/// each group in index order. Rows are filtered on the key's first row
/// before the full key is built.
fn band_groups<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    band: usize,
    changed: &[usize],
) -> Vec<Vec<usize>> {
    let dirty = dirty_band_keys(corpus, band, changed);
    let first_rows: HashSet<u64> = dirty.iter().filter_map(first_row).collect();
    let row = band.saturating_mul(ROWS_PER_BAND);
    let members = (0..corpus.signatures.len())
        .filter_map(|index| Some((index, corpus.signatures.signature(index)?)))
        .filter(|(_, signature)| {
            signature
                .get(row)
                .is_some_and(|value| first_rows.contains(value))
        })
        .map(|(index, signature)| (index, band_key(signature, band)))
        .filter(|(_, key)| dirty.contains(key));
    let mut groups: BTreeMap<[u8; 32], Vec<usize>> = BTreeMap::new();
    for (index, key) in members {
        groups.entry(key).or_default().push(index);
    }
    groups.into_values().collect()
}

/// The `band` keys of the records the edit removed and of every record
/// the changed files now hold.
fn dirty_band_keys<S: BuildHasher>(
    corpus: &EditedCorpus<'_, S>,
    band: usize,
    changed: &[usize],
) -> HashSet<[u8; 32]> {
    let mut dirty: HashSet<[u8; 32]> = corpus.retired.band(band).copied().collect();
    dirty.extend(
        changed
            .iter()
            .filter_map(|index| corpus.signatures.signature(*index))
            .map(|signature| band_key(signature, band)),
    );
    dirty
}

/// A band key's first row, the value its sort tag truncates to.
fn first_row(key: &[u8; 32]) -> Option<u64> {
    key.first_chunk::<8>()
        .map(|bytes| u64::from_le_bytes(*bytes))
}

/// The kept pairs while dirty buckets are re-starred.
struct SourceBook {
    /// Carried pairs, sorted by `(left, right)`.
    carried: Vec<SourcedKey>,
    /// Pairs this edit discovered, with their bits.
    added: HashMap<(usize, usize), u64>,
    /// Keys the gate refused during this edit.
    refused: HashSet<(usize, usize)>,
    /// Records of every dirty bucket.
    members: HashSet<usize>,
    /// Pairs the dirty buckets' stars emitted.
    scanned: u64,
}

impl SourceBook {
    /// A book over the carried pairs, sorted for lookup. Carrying keeps
    /// them in order already; the sort makes lookup independent of that.
    fn new(mut carried: Vec<SourcedKey>) -> Self {
        carried.sort_unstable_by_key(|(left, right, _)| (*left, *right));
        Self {
            carried,
            added: HashMap::new(),
            refused: HashSet::new(),
            members: HashSet::new(),
            scanned: 0,
        }
    }

    /// Re-stars one dirty bucket: its unchanged members' star loses
    /// `source`, then the full star sets it.
    fn restar<S: BuildHasher>(
        &mut self,
        corpus: &EditedCorpus<'_, S>,
        builder: &PairBuilder<'_, S>,
        source: u64,
        group: &[usize],
        rule: StarRule<'_>,
    ) {
        self.members.extend(group);
        let unchanged = corpus.unchanged(group);
        rule(&unchanged, &mut |left, right| {
            self.clear(order(left, right), source);
        });
        rule(group, &mut |left, right| {
            self.scanned = self.scanned.saturating_add(1);
            self.set(builder, order(left, right), source);
        });
    }

    /// The carried row for `key`, if any.
    fn carried_mut(&mut self, key: (usize, usize)) -> Option<&mut SourcedKey> {
        let position = self
            .carried
            .binary_search_by_key(&key, |(left, right, _)| (*left, *right))
            .ok()?;
        self.carried.get_mut(position)
    }

    /// Withdraws `source` from `key`.
    fn clear(&mut self, key: (usize, usize), source: u64) {
        if let Some(row) = self.carried_mut(key) {
            row.2 &= !source;
        } else if let Some(mask) = self.added.get_mut(&key) {
            *mask &= !source;
        }
    }

    /// Records that `source` emits `key`, admitting a key not yet kept.
    fn set<S: BuildHasher>(
        &mut self,
        builder: &PairBuilder<'_, S>,
        key: (usize, usize),
        source: u64,
    ) {
        if let Some(row) = self.carried_mut(key) {
            row.2 |= source;
        } else if let Some(mask) = self.added.get_mut(&key) {
            *mask |= source;
        } else if !self.refused.contains(&key) {
            if builder.admit(key).is_some() {
                let _new = self.added.insert(key, source);
            } else {
                let _new = self.refused.insert(key);
            }
        }
    }

    /// Every pair some source still emits, constructed and in key order.
    fn finish<S: BuildHasher>(
        self,
        builder: &PairBuilder<'_, S>,
        structural_scanned: u64,
    ) -> (SourcedPairs, Vec<usize>) {
        tracing::info!(
            re_paired = self.members.len(),
            structural_scanned,
            "pairs: re-paired after edit"
        );
        let mut re_paired: Vec<usize> = self.members.iter().copied().collect();
        re_paired.sort_unstable();
        let lsh_scanned = self.scanned.saturating_sub(structural_scanned);
        let (pairs, sources): (Vec<CandidatePair>, Vec<u64>) = self
            .live_rows()
            .into_iter()
            .filter_map(|(left, right, mask)| builder.admit((left, right)).map(|pair| (pair, mask)))
            .unzip();
        tracing::info!(
            rss_mib = crate::observe::resident_mib(),
            kept = pairs.len(),
            lsh_scanned,
            "pairs: post-lsh"
        );
        (SourcedPairs { pairs, sources }, re_paired)
    }

    /// The rows some source still emits, in key order.
    fn live_rows(self) -> Vec<SourcedKey> {
        let added = self
            .added
            .into_iter()
            .map(|((left, right), mask)| (left, right, mask));
        let mut rows: Vec<SourcedKey> = self
            .carried
            .into_iter()
            .chain(added)
            .filter(|(_, _, mask)| *mask != 0)
            .collect();
        rows.sort_unstable_by_key(|(left, right, _)| (*left, *right));
        rows
    }
}
