//! Embedding-pass batch data and mapping helpers.

use std::collections::HashMap;

use crate::{
    embedding::{embedding_pairs, EmbeddingPair, EmbeddingSpec},
    fingerprint::Fingerprint,
    report::EmbeddingProvenance,
    state::FileId,
};

/// Accumulates successful vectors and rejected occurrence counts.
#[derive(Debug)]
pub(super) struct EmbeddingBatch {
    /// Successful vectors keyed by original fingerprint index.
    pub(super) vectors: Vec<IndexedEmbedding>,
    /// Logical occurrences represented by successful vectors.
    successes: usize,
    /// Logical occurrences skipped because the provider rejected them.
    pub(super) failures: usize,
}

impl EmbeddingBatch {
    /// Creates an empty batch with space for expected successes.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            vectors: Vec::with_capacity(capacity),
            successes: 0,
            failures: 0,
        }
    }

    /// Adds one successful embedding vector.
    pub(super) fn push(&mut self, fingerprint_index: usize, vector: Vec<f32>, occurrences: usize) {
        self.vectors.push(IndexedEmbedding {
            fingerprint_index,
            vector,
        });
        self.successes = self.successes.saturating_add(occurrences);
    }

    /// Returns successful vectors plus rejected occurrences.
    pub(super) fn processed(&self) -> usize {
        self.successes.saturating_add(self.failures)
    }
}

/// Provider request waiting to be embedded.
#[derive(Debug)]
pub(super) struct PendingEmbedding {
    /// Original fingerprint index represented by this request.
    pub(super) fingerprint_index: usize,
    /// Source text sent to the provider.
    pub(super) snippet: String,
    /// Stable content hash used for cache writes and diagnostics.
    pub(super) snippet_hash: String,
    /// Logical duplicate occurrences represented by this snippet.
    pub(super) occurrences: usize,
}

/// Successful vector tied to its original fingerprint index.
#[derive(Debug)]
pub(super) struct IndexedEmbedding {
    /// Original fingerprint index.
    fingerprint_index: usize,
    /// Provider-returned vector.
    vector: Vec<f32>,
}

/// Builds ANN pairs from successfully embedded snippets.
pub(super) fn pairs_from_successful_embeddings(
    fingerprints: &[Fingerprint],
    indexed: &[IndexedEmbedding],
) -> Vec<EmbeddingPair> {
    let successful_fingerprints: Vec<Fingerprint> = indexed
        .iter()
        .filter_map(|item| fingerprints.get(item.fingerprint_index).cloned())
        .collect();
    let vectors: Vec<Vec<f32>> = indexed.iter().map(|item| item.vector.clone()).collect();
    embedding_pairs(&successful_fingerprints, &vectors)
        .into_iter()
        .filter_map(|pair| remap_pair(pair, indexed))
        .collect()
}

/// Returns the source slice for `fingerprint` as a `String`.
pub(super) fn snippet_for(fingerprint: &Fingerprint, sources: &HashMap<FileId, Vec<u8>>) -> String {
    let Some(bytes) = sources.get(&fingerprint.file_id) else {
        return String::new();
    };
    let start = fingerprint.byte_range.start.min(bytes.len());
    let end = fingerprint.byte_range.end.min(bytes.len());
    bytes
        .get(start..end)
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .unwrap_or_default()
}

/// Lifts an [`EmbeddingSpec`] into the report-facing provenance struct.
pub(super) fn provenance_from(
    spec: EmbeddingSpec,
    attempted_subtrees: usize,
    indexed_subtrees: usize,
    failed_subtrees: usize,
) -> EmbeddingProvenance {
    EmbeddingProvenance {
        provider_id: spec.provider_id,
        model_id: spec.model_id,
        model_version: spec.model_version,
        dimensions: spec.dimensions,
        attempted_subtrees,
        indexed_subtrees,
        failed_subtrees,
    }
}

/// Maps compact pair indices back to full fingerprint indices.
fn remap_pair(pair: EmbeddingPair, indexed: &[IndexedEmbedding]) -> Option<EmbeddingPair> {
    let left = indexed.get(pair.left)?.fingerprint_index;
    let right = indexed.get(pair.right)?.fingerprint_index;
    Some(EmbeddingPair {
        left,
        right,
        cosine: pair.cosine,
    })
}
