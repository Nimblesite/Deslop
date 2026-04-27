//! Embedding pass orchestration shared by [`super::run`] and
//! [`super::session`]. Reads the corpus, dispatches the
//! [`EmbeddingProvider`] based on the run's [`EmbeddingMode`], and
//! returns the ANN-nearest-neighbour pairs plus report provenance.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    thread,
    time::Duration,
};

use crate::{
    embedding::{
        cache::DEFAULT_CACHE_DIR_NAME, content_hash, embedding_pairs, EmbeddingCache,
        EmbeddingMode, EmbeddingPair, EmbeddingProvider, EmbeddingSpec, ProviderError,
    },
    error::CoreError,
    fingerprint::Fingerprint,
    report::EmbeddingProvenance,
    state::FileId,
};

use super::{config::PipelineConfig, corpus::FingerprintCorpus};

/// Outcome of the embedding pass. Empty `pairs` + `None` provenance
/// means the pass was skipped or failed gracefully.
#[derive(Debug, Default)]
pub struct EmbeddingOutcome {
    /// ANN-nearest-neighbour pairs produced by the embedding pass.
    pub pairs: Vec<EmbeddingPair>,
    /// Provenance to record in the rendered report.
    pub provenance: Option<EmbeddingProvenance>,
}

/// Runs the embedding pass honouring `config.embedding.mode`:
///
/// - `Off` → skip entirely.
/// - `Auto` → try; on failure log a warning and continue with empty
///   pairs.
/// - `Required` → try; on failure propagate so the CLI exits
///   non-zero.
///
/// # Errors
///
/// Returns [`CoreError::Embedding`] when the provider fails and
/// `mode` is [`EmbeddingMode::Required`]. Auto-mode failures log and
/// return an empty outcome.
pub fn run_embedding_pass(
    config: &PipelineConfig<'_>,
    corpus: &FingerprintCorpus,
) -> Result<EmbeddingOutcome, CoreError> {
    if matches!(config.embedding.mode, EmbeddingMode::Off) {
        return Ok(EmbeddingOutcome::default());
    }
    match embed_corpus(config, corpus) {
        Ok(outcome) => Ok(outcome),
        Err(source) if matches!(config.embedding.mode, EmbeddingMode::Auto) => {
            tracing::warn!(error = %source, "embedding pass unavailable — continuing without Type-4 recall");
            Ok(EmbeddingOutcome::default())
        }
        Err(source) => Err(source),
    }
}

/// Actually runs the embedding pass. The caller has already
/// guaranteed `mode != Off`; a `None` provider here is a caller bug
/// and produces an empty outcome defensively.
fn embed_corpus(
    config: &PipelineConfig<'_>,
    corpus: &FingerprintCorpus,
) -> Result<EmbeddingOutcome, CoreError> {
    let Some(provider) = config.embedding.provider else {
        return Ok(EmbeddingOutcome::default());
    };
    provider.probe().map_err(|source| CoreError::Embedding {
        message: source.to_string(),
    })?;
    let spec = provider.spec();
    tracing::info!(
        provider = %spec.provider_id,
        model = %spec.model_id,
        version = %spec.model_version,
        dims = spec.dimensions,
        subtrees = corpus.fingerprints.len(),
        "embedding pass starting",
    );
    let cache = open_cache(&config.root, &spec)?;
    let batch = compute_embeddings(
        provider,
        &cache,
        corpus,
        spec.dimensions,
        config.embedding.batch_yield,
        config.embedding.progress,
    );
    let pairs = pairs_from_successful_embeddings(&corpus.fingerprints, &batch.vectors);
    tracing::info!(
        pair_count = pairs.len(),
        embedded = batch.vectors.len(),
        failed = batch.failures,
        "embedding pass complete"
    );
    Ok(EmbeddingOutcome {
        pairs,
        provenance: Some(provenance_from(
            spec,
            corpus.fingerprints.len(),
            batch.vectors.len(),
            batch.failures,
        )),
    })
}

/// Opens the on-disk embedding cache under the scan root. Swallows
/// the I/O error with a `CoreError::Embedding` — if the cache
/// directory cannot be created the whole pass is degraded.
fn open_cache(scan_root: &Path, spec: &EmbeddingSpec) -> Result<EmbeddingCache, CoreError> {
    let base = scan_root.join(DEFAULT_CACHE_DIR_NAME);
    EmbeddingCache::open(&base, spec).map_err(|source| CoreError::Embedding {
        message: format!("open embedding cache: {source}"),
    })
}

/// Produces embedding vectors for fingerprints whose provider request
/// succeeds. Cache hits short-circuit the provider call; misses invoke
/// the provider and persist the result for subsequent runs.
fn compute_embeddings(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    corpus: &FingerprintCorpus,
    dimensions: usize,
    batch_yield: Option<Duration>,
    progress: Option<&dyn Fn(usize)>,
) -> EmbeddingBatch {
    let mut batch = EmbeddingBatch::with_capacity(corpus.fingerprints.len());
    let mut indexed_hashes: HashSet<String> = HashSet::new();
    let mut pending_positions: HashMap<String, usize> = HashMap::new();
    let mut pending: Vec<PendingEmbedding> = Vec::new();
    for (index, fingerprint) in corpus.fingerprints.iter().enumerate() {
        let snippet = snippet_for(fingerprint, &corpus.sources);
        let snippet_hash = content_hash(&snippet);
        if indexed_hashes.contains(&snippet_hash) {
            continue;
        }
        if let Some(position) = pending_positions.get(&snippet_hash).copied() {
            if let Some(queued) = pending.get_mut(position) {
                queued.occurrences = queued.occurrences.saturating_add(1);
            }
            continue;
        }
        if let Some(cached) = cache.get(&snippet) {
            let _inserted = indexed_hashes.insert(snippet_hash);
            batch.push(index, cached);
            continue;
        }
        let _previous = pending_positions.insert(snippet_hash.clone(), pending.len());
        pending.push(PendingEmbedding {
            fingerprint_index: index,
            snippet,
            snippet_hash,
            occurrences: 1,
        });
    }
    process_pending_embeddings(
        provider,
        cache,
        &mut batch,
        &pending,
        dimensions,
        batch_yield,
        progress,
    );
    if batch.failures > 0 {
        tracing::warn!(
            failed = batch.failures,
            total = corpus.fingerprints.len(),
            "embedding pass completed with rejected subtrees"
        );
    }
    batch
}

/// Dispatches pending embedding requests in provider-sized chunks.
fn process_pending_embeddings(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    pending: &[PendingEmbedding],
    dimensions: usize,
    batch_yield: Option<Duration>,
    progress: Option<&dyn Fn(usize)>,
) {
    let max_batch_size = provider.max_batch_size().max(1);
    for (index, chunk) in pending.chunks(max_batch_size).enumerate() {
        embed_chunk(provider, cache, batch, chunk, dimensions);
        report_progress(progress, batch);
        maybe_yield_between_batches(batch_yield, index, pending.len(), max_batch_size);
    }
}

/// Embeds one chunk, splitting failed multi-input requests so a
/// context error on an aggregate Ollama request does not discard small
/// snippets that succeed on their own.
fn embed_chunk(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    chunk: &[PendingEmbedding],
    dimensions: usize,
) {
    let inputs: Vec<String> = chunk.iter().map(|item| item.snippet.clone()).collect();
    match provider.embed_batch(&inputs) {
        Ok(vectors) if vectors.len() == chunk.len() => {
            push_fresh_embeddings(cache, batch, chunk, vectors, dimensions);
        }
        Ok(vectors) => record_bad_vector_count(batch, chunk, vectors.len()),
        Err(source) if chunk.len() > 1 => {
            split_and_retry(provider, cache, batch, chunk, dimensions, &source);
        }
        Err(source) => record_failed_chunk(batch, chunk, &source),
    }
}

/// Stores all successful vectors from one provider response.
fn push_fresh_embeddings(
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    chunk: &[PendingEmbedding],
    vectors: Vec<Vec<f32>>,
    dimensions: usize,
) {
    for (item, vector) in chunk.iter().zip(vectors) {
        push_fresh_embedding(cache, batch, item, vector, dimensions);
    }
}

/// Records a malformed response that did not preserve batch arity.
fn record_bad_vector_count(batch: &mut EmbeddingBatch, chunk: &[PendingEmbedding], actual: usize) {
    let message = format!("expected {} embeddings, got {actual}", chunk.len());
    record_failed_chunk(batch, chunk, &message);
}

/// Bisects a failed provider batch and retries both halves.
fn split_and_retry(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    chunk: &[PendingEmbedding],
    dimensions: usize,
    source: &ProviderError,
) {
    tracing::debug!(
        error = %source,
        inputs = chunk.len(),
        "embedding batch failed; retrying smaller chunks"
    );
    let mid = chunk.len() / 2;
    if let (Some(left), Some(right)) = (chunk.get(..mid), chunk.get(mid..)) {
        embed_chunk(provider, cache, batch, left, dimensions);
        embed_chunk(provider, cache, batch, right, dimensions);
    }
}

/// Reports processed embedding count to the optional progress sink.
fn report_progress(progress: Option<&dyn Fn(usize)>, batch: &EmbeddingBatch) {
    if let Some(progress) = progress {
        progress(batch.processed());
    }
}

/// Sleeps between provider chunks when a caller requested cooperative
/// yielding.
fn maybe_yield_between_batches(
    batch_yield: Option<Duration>,
    chunk_index: usize,
    pending_len: usize,
    max_batch_size: usize,
) {
    let Some(delay) = batch_yield.filter(|delay| !delay.is_zero()) else {
        return;
    };
    let next_chunk = chunk_index.saturating_add(1);
    if next_chunk < pending_len.div_ceil(max_batch_size) {
        thread::sleep(delay);
    }
}

/// Stores one successful provider vector when its dimensions match.
fn push_fresh_embedding(
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    item: &PendingEmbedding,
    vector: Vec<f32>,
    dimensions: usize,
) {
    if vector.len() != dimensions {
        let message = format!("expected {dimensions} dims, got {}", vector.len());
        record_failed_pending(batch, item, &message);
        return;
    }
    if let Err(error) = cache.store(&item.snippet, &vector) {
        tracing::warn!(%error, content_hash = %item.snippet_hash, "embedding cache write failed");
    }
    batch.push(item.fingerprint_index, vector);
}

/// Records every pending item in a failed provider batch.
fn record_failed_chunk<E: std::fmt::Display>(
    batch: &mut EmbeddingBatch,
    chunk: &[PendingEmbedding],
    error: &E,
) {
    for item in chunk {
        record_failed_pending(batch, item, error);
    }
}

/// Records one failed pending embedding request.
fn record_failed_pending<E: std::fmt::Display>(
    batch: &mut EmbeddingBatch,
    item: &PendingEmbedding,
    error: &E,
) {
    batch.failures = batch.failures.saturating_add(item.occurrences);
    tracing::warn!(
        error = %error,
        occurrences = item.occurrences,
        snippet_chars = item.snippet.chars().count(),
        content_hash = %item.snippet_hash,
        "embedding provider rejected subtree — skipping embedding signal"
    );
}

#[derive(Debug)]
/// Accumulates successful vectors and rejected occurrence counts.
struct EmbeddingBatch {
    /// Successful vectors keyed by original fingerprint index.
    vectors: Vec<IndexedEmbedding>,
    /// Logical occurrences skipped because the provider rejected them.
    failures: usize,
}

impl EmbeddingBatch {
    /// Creates an empty batch with space for expected successes.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            vectors: Vec::with_capacity(capacity),
            failures: 0,
        }
    }

    /// Adds one successful embedding vector.
    fn push(&mut self, fingerprint_index: usize, vector: Vec<f32>) {
        self.vectors.push(IndexedEmbedding {
            fingerprint_index,
            vector,
        });
    }

    /// Returns successful vectors plus rejected occurrences.
    fn processed(&self) -> usize {
        self.vectors.len().saturating_add(self.failures)
    }
}

#[derive(Debug)]
/// Provider request waiting to be embedded.
struct PendingEmbedding {
    /// Original fingerprint index represented by this request.
    fingerprint_index: usize,
    /// Source text sent to the provider.
    snippet: String,
    /// Stable content hash used for cache writes and diagnostics.
    snippet_hash: String,
    /// Logical duplicate occurrences represented by this snippet.
    occurrences: usize,
}

#[derive(Debug)]
/// Successful vector tied to its original fingerprint index.
struct IndexedEmbedding {
    /// Original fingerprint index.
    fingerprint_index: usize,
    /// Provider-returned vector.
    vector: Vec<f32>,
}

/// Builds ANN pairs from successfully embedded snippets.
fn pairs_from_successful_embeddings(
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

/// Maps pair indices from the compact embedded set back to the full
/// fingerprint list.
fn remap_pair(pair: EmbeddingPair, indexed: &[IndexedEmbedding]) -> Option<EmbeddingPair> {
    let left = indexed.get(pair.left)?.fingerprint_index;
    let right = indexed.get(pair.right)?.fingerprint_index;
    Some(EmbeddingPair {
        left,
        right,
        cosine: pair.cosine,
    })
}

/// Returns the source slice for `fingerprint` as a `String`. Invalid
/// byte ranges (impossible in the current pipeline) collapse to an
/// empty string, which the provider then embeds as a constant vector
/// — keeps the helper total without a branch in the caller.
fn snippet_for(fingerprint: &Fingerprint, sources: &HashMap<FileId, Vec<u8>>) -> String {
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

/// Lifts an [`EmbeddingSpec`] into the report-facing
/// [`EmbeddingProvenance`] struct.
fn provenance_from(
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
