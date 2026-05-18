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
        cache::DEFAULT_CACHE_DIR_NAME, content_hash, EmbeddingCache, EmbeddingMode, EmbeddingPair,
        EmbeddingProvider, EmbeddingSpec, ProviderError,
    },
    error::CoreError,
    report::EmbeddingProvenance,
};

use super::{
    config::PipelineConfig,
    corpus::FingerprintCorpus,
    embedding_batch::{
        pairs_from_successful_embeddings, provenance_from, snippet_for, EmbeddingBatch,
        PendingEmbedding,
    },
    embedding_observability::{token_count, EmbeddingObserver},
};

/// Maximum source characters sent to any embedding provider.
const MAX_PROVIDER_INPUT_CHARS: usize = 6_000;

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
    let mut observer = EmbeddingObserver::new(corpus.fingerprints.len());
    let batch = compute_embeddings(
        provider,
        &cache,
        corpus,
        spec.dimensions,
        config.embedding.batch_yield,
        config.embedding.progress,
        &mut observer,
    );
    let pairs = pairs_from_successful_embeddings(&corpus.fingerprints, &batch.vectors);
    observer.log_final(pairs.len(), batch.vectors.len(), batch.failures);
    Ok(EmbeddingOutcome {
        pairs,
        provenance: Some(provenance_from(
            spec,
            attempted_subtrees(corpus.fingerprints.len(), &batch),
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

/// Produces embedding vectors for fingerprints whose provider request succeeds.
fn compute_embeddings(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    corpus: &FingerprintCorpus,
    dimensions: usize,
    batch_yield: Option<Duration>,
    progress: Option<&dyn Fn(usize)>,
    observer: &mut EmbeddingObserver,
) -> EmbeddingBatch {
    let mut batch = EmbeddingBatch::with_capacity(corpus.fingerprints.len());
    let pending = lookup_phase(corpus, cache, &mut batch, observer);
    observer.log_cache_phase(pending.len());
    process_pending_embeddings(
        PendingDispatch {
            provider,
            cache,
            dimensions,
            batch_yield,
            progress,
            observer,
        },
        &mut batch,
        &pending,
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

/// Walks the corpus once, loading cache hits and queuing unique misses.
fn lookup_phase(
    corpus: &FingerprintCorpus,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    observer: &mut EmbeddingObserver,
) -> Vec<PendingEmbedding> {
    let mut indexed_hashes: HashSet<String> = HashSet::new();
    let mut pending_positions: HashMap<String, usize> = HashMap::new();
    let mut pending: Vec<PendingEmbedding> = Vec::new();
    for (index, fingerprint) in corpus.fingerprints.iter().enumerate() {
        let snippet = snippet_for(fingerprint, &corpus.sources);
        if snippet.chars().count() > MAX_PROVIDER_INPUT_CHARS {
            record_oversized_input(batch, index, snippet);
            continue;
        }
        classify_snippet(
            ClassifyContext {
                index,
                snippet,
                cache,
                batch,
                observer,
            },
            &mut indexed_hashes,
            &mut pending_positions,
            &mut pending,
        );
    }
    pending
}

/// Inputs for [`classify_snippet`].
struct ClassifyContext<'a> {
    /// Position of the fingerprint inside the corpus.
    index: usize,
    /// Source slice extracted for this fingerprint.
    snippet: String,
    /// Embedding cache consulted for warm-cache short-circuits.
    cache: &'a EmbeddingCache,
    /// Mutable batch where cache hits are recorded immediately.
    batch: &'a mut EmbeddingBatch,
    /// Pass-level observer updated on hit, miss, or duplicate.
    observer: &'a mut EmbeddingObserver,
}

/// Routes one snippet onto cache-hit, dedup-merge, or queue-pending.
fn classify_snippet(
    ctx: ClassifyContext<'_>,
    indexed_hashes: &mut HashSet<String>,
    pending_positions: &mut HashMap<String, usize>,
    pending: &mut Vec<PendingEmbedding>,
) {
    let snippet_hash = content_hash(&ctx.snippet);
    if indexed_hashes.contains(&snippet_hash) {
        ctx.observer.record_duplicate();
        return;
    }
    if let Some(position) = pending_positions.get(&snippet_hash).copied() {
        if let Some(queued) = pending.get_mut(position) {
            queued.occurrences = queued.occurrences.saturating_add(1);
        }
        ctx.observer.record_duplicate();
        return;
    }
    if let Some(cached) = ctx.cache.get(&ctx.snippet) {
        let _inserted = indexed_hashes.insert(snippet_hash);
        ctx.batch.push(ctx.index, cached, 1);
        ctx.observer.record_cache_hit();
        return;
    }
    ctx.observer.record_cache_miss();
    let _previous = pending_positions.insert(snippet_hash.clone(), pending.len());
    pending.push(PendingEmbedding {
        fingerprint_index: ctx.index,
        snippet: ctx.snippet,
        snippet_hash,
        occurrences: 1,
    });
}

/// Pending provider-dispatch dependencies.
struct PendingDispatch<'a> {
    /// Embedding provider receiving each batch.
    provider: &'a dyn EmbeddingProvider,
    /// On-disk cache that absorbs successful vectors.
    cache: &'a EmbeddingCache,
    /// Provider-spec embedding dimensionality.
    dimensions: usize,
    /// Optional cooperative yield between batches.
    batch_yield: Option<Duration>,
    /// Optional live progress callback.
    progress: Option<&'a dyn Fn(usize)>,
    /// Pass-level structured observer.
    observer: &'a mut EmbeddingObserver,
}

/// Returns the provenance denominator for this embedding pass.
fn attempted_subtrees(total_fingerprints: usize, batch: &EmbeddingBatch) -> usize {
    if batch.failures == 0 {
        return total_fingerprints;
    }
    batch.vectors.len().saturating_add(batch.failures)
}

/// Counts an oversized snippet as skipped before provider dispatch.
fn record_oversized_input(batch: &mut EmbeddingBatch, fingerprint_index: usize, snippet: String) {
    let snippet_hash = content_hash(&snippet);
    let item = PendingEmbedding {
        fingerprint_index,
        snippet,
        snippet_hash,
        occurrences: 1,
    };
    record_failed_pending(
        batch,
        &item,
        &format!("exceeds {MAX_PROVIDER_INPUT_CHARS} chars"),
    );
}

/// Dispatches pending embedding requests in provider-sized chunks.
fn process_pending_embeddings(
    dispatch: PendingDispatch<'_>,
    batch: &mut EmbeddingBatch,
    pending: &[PendingEmbedding],
) {
    let PendingDispatch {
        provider,
        cache,
        dimensions,
        batch_yield,
        progress,
        observer,
    } = dispatch;
    let max_batch_size = provider.max_batch_size().max(1);
    let total_batches = pending.len().div_ceil(max_batch_size);
    for (index, chunk) in pending.chunks(max_batch_size).enumerate() {
        let batch_index = index.saturating_add(1);
        let tokens = provider_batch_tokens(chunk);
        observer.provider_batch(batch_index, total_batches, chunk.len(), tokens, || {
            embed_chunk(provider, cache, batch, chunk, dimensions);
        });
        report_progress(progress, batch);
        maybe_yield_between_batches(batch_yield, index, total_batches);
    }
}

/// Returns the approximate token count for one provider batch.
fn provider_batch_tokens(chunk: &[PendingEmbedding]) -> usize {
    chunk.iter().fold(0, |total, item| {
        total.saturating_add(token_count(&item.snippet))
    })
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
    total_batches: usize,
) {
    let Some(delay) = batch_yield.filter(|delay| !delay.is_zero()) else {
        return;
    };
    let next_chunk = chunk_index.saturating_add(1);
    if next_chunk < total_batches {
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
    batch.push(item.fingerprint_index, vector, item.occurrences);
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
