//! [PIPELINE-INCREMENTAL-ANALYSIS-PAIRS] The pairs the last render kept, carried into the
//! next live pass so an edit re-pairs only the buckets it touched.
//!
//! The ledger holds keys and source bits only, never candidates: a
//! candidate is rebuilt from its endpoints, which is pure. It is dropped
//! whenever the comparison space changes shape (a config or ignore-rule
//! reload), and never built while embeddings or the cross-language audit
//! add pairs no bucket explains; those passes pair the whole corpus.

use std::collections::{HashMap, HashSet};

use super::PipelineSession;
use crate::{
    fingerprint::Fingerprint,
    lsh::{BandCollisionSource, SignatureIndex},
    pair::{
        candidate_pairs_after_edit, sourced_candidate_pairs, CandidatePair, EditedCorpus,
        RetiredKeys, SourcedKey, SourcedPairs,
    },
    state::FileId,
};

/// One live file's span of the flat records.
#[derive(Debug, Clone, Copy)]
pub(super) struct FileSpan {
    /// The file.
    pub(super) file_id: FileId,
    /// Index of its first record.
    pub(super) offset: usize,
    /// Its record count.
    pub(super) count: usize,
}

/// What the store changed since the last render took its changes.
#[derive(Debug, Default)]
pub(super) struct StoreChanges {
    /// Files whose records were replaced, added or removed.
    pub(super) touched: HashSet<FileId>,
    /// Keys of the records removed.
    pub(super) retired: RetiredKeys,
}

impl StoreChanges {
    /// Records one removed record of `file_id`.
    pub(super) fn retire(
        &mut self,
        file_id: FileId,
        fingerprint: &Fingerprint,
        signature: Option<&crate::lsh::Signature>,
    ) {
        let _new = self.touched.insert(file_id);
        self.retired.retire(fingerprint, signature);
    }
}

/// The kept pairs of one render, in that render's record layout.
#[derive(Debug, Default)]
pub(super) struct PairLedger {
    /// The layout the pair indices refer to.
    spans: Vec<FileSpan>,
    /// Kept pairs with their source bits, sorted.
    pairs: Vec<SourcedKey>,
}

impl PairLedger {
    /// The ledger of `pairs` in layout `spans`.
    fn new(spans: Vec<FileSpan>, pairs: &[CandidatePair], sources: &[u64]) -> Self {
        Self {
            spans,
            pairs: pairs
                .iter()
                .zip(sources)
                .map(|(pair, source)| (pair.left, pair.right, *source))
                .collect(),
        }
    }

    /// The pairs between unchanged files, re-indexed into layout `now`.
    fn carry(self, now: &[FileSpan], changed: &HashSet<FileId>) -> Vec<SourcedKey> {
        let offsets: HashMap<FileId, usize> =
            now.iter().map(|span| (span.file_id, span.offset)).collect();
        let remap = |index: usize| {
            let span = locate(&self.spans, index)?;
            let offset = offsets
                .get(&span.file_id)
                .filter(|_| !changed.contains(&span.file_id))?;
            Some(offset.saturating_add(index.saturating_sub(span.offset)))
        };
        self.pairs
            .iter()
            .filter_map(|&(left, right, source)| Some((remap(left)?, remap(right)?, source)))
            .collect()
    }
}

/// The span holding record `index`.
fn locate(spans: &[FileSpan], index: usize) -> Option<&FileSpan> {
    let position = spans.partition_point(|span| span.offset.saturating_add(span.count) <= index);
    spans.get(position).filter(|span| span.offset <= index)
}

/// The kept pairs of one live pass, the ledger the next pass carries,
/// and how many records were fed to pairing.
pub(super) struct LivePairs {
    /// The kept candidates in key order.
    pub(super) pairs: Vec<CandidatePair>,
    /// The ledger for the next pass.
    pub(super) ledger: PairLedger,
    /// Records whose pairs were (re)computed.
    pub(super) fed: usize,
}

impl PipelineSession {
    /// [PIPELINE-INCREMENTAL-ANALYSIS-PAIRS] Pairs the corpus: from the previous ledger and
    /// the store's changes when there is one, else from scratch.
    pub(super) fn live_pairs(
        &self,
        previous: Option<PairLedger>,
        changes: &StoreChanges,
        signatures: &SignatureIndex<'_>,
    ) -> LivePairs {
        let spans = self.store.spans();
        let (sourced, fed) = previous.map_or_else(
            || self.pairs_from_scratch(signatures),
            |ledger| {
                self.pairs_after_edit(ledger.carry(&spans, &changes.touched), changes, signatures)
            },
        );
        let ledger = PairLedger::new(spans, &sourced.pairs, &sourced.sources);
        LivePairs {
            pairs: sourced.pairs,
            ledger,
            fed,
        }
    }

    /// Every bucket paired, with source bits.
    fn pairs_from_scratch(&self, signatures: &SignatureIndex<'_>) -> (SourcedPairs, usize) {
        let lsh = BandCollisionSource::new(signatures);
        let fingerprints = self.store.fingerprints();
        let sourced = sourced_candidate_pairs(fingerprints, signatures, &lsh, &self.file_languages);
        (sourced, signatures.len())
    }

    /// The dirty buckets re-paired over the carried pairs.
    fn pairs_after_edit(
        &self,
        carried: Vec<SourcedKey>,
        changes: &StoreChanges,
        signatures: &SignatureIndex<'_>,
    ) -> (SourcedPairs, usize) {
        let corpus = EditedCorpus {
            fingerprints: self.store.fingerprints(),
            signatures,
            file_languages: &self.file_languages,
            changed: &changes.touched,
            retired: &changes.retired,
        };
        candidate_pairs_after_edit(&corpus, carried)
    }

    /// Pairs the whole corpus under the language policy, embedding and
    /// cross-language pairs included.
    pub(super) fn whole_corpus_pairs(
        &self,
        signatures: &crate::lsh::SignatureIndex<'_>,
        trees: Option<&[crate::ast::NormalizedNode]>,
        embedding_outcome: &crate::pipeline::embedding_pass::EmbeddingOutcome,
    ) -> Vec<crate::pair::CandidatePair> {
        let fingerprints = self.store.fingerprints();
        let cross_language_signatures = trees.map(|trees| {
            crate::pipeline::signatures::build_cross_language_signatures(
                fingerprints,
                trees,
                &self.file_languages,
            )
        });
        tracing::debug!(
            signatures = signatures.len(),
            "streaming LSH band collisions"
        );
        crate::pair::candidate_pairs_for_language_policy(
            fingerprints,
            signatures,
            &BandCollisionSource::new(signatures),
            &embedding_outcome.pairs,
            cross_language_signatures.as_deref(),
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        )
    }
}
