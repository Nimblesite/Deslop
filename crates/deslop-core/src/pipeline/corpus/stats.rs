//! Corpus-build observability counters
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE],
//! [PIPELINE-OBSERVABILITY-STAGES]).
//!
//! One accumulator threaded through the corpus build so the
//! `fingerprint corpus built` event can attribute the stage's wall time
//! to its substages — read, parse+normalise, fingerprint collection,
//! signature construction — and split the fingerprint population into
//! exact-node and sibling-window counts. Observability only —
//! deliberately not part of the wire-model [`crate::report::CacheStats`],
//! which is generated from the typeDiagram IPC contract.

use std::time::Duration;

use crate::observe::duration_ms;

/// Per-run corpus-build counters surfaced on the `fingerprint corpus
/// built` event and the fixed-interval progress records.
#[derive(Debug, Default)]
pub struct CorpusBuildStats {
    /// Signatures constructed from token streams this pass.
    pub signatures_built: u64,
    /// Signatures attached from the on-disk parse store this pass.
    pub signatures_reused: u64,
    /// Exact-node structural fingerprints collected this pass.
    pub exact_fingerprints: u64,
    /// Synthetic sibling-window fingerprints collected this pass.
    pub sibling_fingerprints: u64,
    /// Accumulated source-file read time.
    read: Duration,
    /// Accumulated parse + normalise time.
    parse: Duration,
    /// Accumulated fingerprint-collection time (both families).
    fingerprint: Duration,
    /// Accumulated signature-construction time (token resolution +
    /// `MinHash`).
    signature: Duration,
}

impl CorpusBuildStats {
    /// Records `count` signatures constructed from token streams.
    pub fn add_built(&mut self, count: usize) {
        self.signatures_built = self.signatures_built.saturating_add(saturated(count));
    }

    /// Records `count` signatures attached from the parse store.
    pub fn add_reused(&mut self, count: usize) {
        self.signatures_reused = self.signatures_reused.saturating_add(saturated(count));
    }

    /// Records one file's fingerprint population split by family.
    pub fn add_fingerprint_kinds(&mut self, exact: usize, sibling: usize) {
        self.exact_fingerprints = self.exact_fingerprints.saturating_add(saturated(exact));
        self.sibling_fingerprints = self.sibling_fingerprints.saturating_add(saturated(sibling));
    }

    /// Accumulates source-read time.
    pub fn add_read(&mut self, elapsed: Duration) {
        self.read = self.read.saturating_add(elapsed);
    }

    /// Accumulates parse + normalise time.
    pub fn add_parse(&mut self, elapsed: Duration) {
        self.parse = self.parse.saturating_add(elapsed);
    }

    /// Accumulates fingerprint-collection time.
    pub fn add_fingerprint(&mut self, elapsed: Duration) {
        self.fingerprint = self.fingerprint.saturating_add(elapsed);
    }

    /// Accumulates signature-construction time.
    pub fn add_signature(&mut self, elapsed: Duration) {
        self.signature = self.signature.saturating_add(elapsed);
    }

    /// Accumulated read milliseconds.
    #[must_use]
    pub fn read_ms(&self) -> u64 {
        duration_ms(self.read)
    }

    /// Accumulated parse + normalise milliseconds.
    #[must_use]
    pub fn parse_ms(&self) -> u64 {
        duration_ms(self.parse)
    }

    /// Accumulated fingerprint-collection milliseconds.
    #[must_use]
    pub fn fingerprint_ms(&self) -> u64 {
        duration_ms(self.fingerprint)
    }

    /// Accumulated signature-construction milliseconds.
    #[must_use]
    pub fn signature_ms(&self) -> u64 {
        duration_ms(self.signature)
    }
}

/// Saturating `usize → u64` for counter arithmetic.
fn saturated(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}
