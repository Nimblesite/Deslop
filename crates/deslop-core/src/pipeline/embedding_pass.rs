//! Embedding pass orchestration shared by [`super::run`] and
//! [`super::session`]. Reads the corpus, dispatches the
//! [`EmbeddingProvider`] based on the run's [`EmbeddingMode`], and
//! returns the ANN-nearest-neighbour pairs plus report provenance.

use std::{collections::HashMap, path::Path, thread, time::Duration};

use crate::{
    embedding::{
        content_hash, EmbeddingCache, EmbeddingMode, EmbeddingPair, EmbeddingProvider,
        EmbeddingSpec, ProviderError,
    },
    error::CoreError,
    fingerprint::Fingerprint,
    report::EmbeddingProvenance,
    state::FileId,
};

use super::{
    config::PipelineConfig,
    embedding_batch::{
        pairs_from_successful_embeddings, provenance_from, snippet_for, vectors_by_fingerprint,
        EmbeddingBatch, PendingEmbedding,
    },
    embedding_observability::{token_count, EmbeddingObserver},
};

/// Outcome of the embedding pass. Empty `pairs` + `None` provenance
/// means the pass was skipped or failed gracefully.
#[derive(Debug, Default)]
pub struct EmbeddingOutcome {
    /// ANN-nearest-neighbour pairs produced by the embedding pass.
    pub pairs: Vec<EmbeddingPair>,
    /// Every successfully embedded vector, keyed by fingerprint index.
    ///
    /// Cluster materialisation measures `embedding_cos` between the
    /// occurrences it actually renders, which needs the vectors — the
    /// ANN pair list alone only covers the neighbours the index
    /// surfaced. Empty when the pass was skipped or failed gracefully.
    pub vectors: HashMap<usize, Vec<f32>>,
    /// Provenance to record in the rendered report.
    pub provenance: Option<EmbeddingProvenance>,
}

/// Borrowed view of the corpus consumed by the embedding pass: the
/// flat fingerprint slice plus the per-file source bytes behind it.
/// Borrowed straight from the session's canonical storage so the pass
/// copies no corpus state ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
pub struct CorpusView<'a> {
    /// Every live fingerprint, flat, in corpus order.
    pub fingerprints: &'a [Fingerprint],
    /// Source bytes keyed by the file id each fingerprint references.
    pub sources: &'a HashMap<FileId, Vec<u8>>,
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
    corpus: &CorpusView<'_>,
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
    corpus: &CorpusView<'_>,
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
    let pairs = pairs_from_successful_embeddings(corpus.fingerprints, &batch.vectors);
    observer.log_final(pairs.len(), batch.vectors.len(), batch.failures);
    let provenance = provenance_from(spec, &batch);
    Ok(EmbeddingOutcome {
        pairs,
        vectors: vectors_by_fingerprint(batch.vectors),
        provenance: Some(provenance),
    })
}

/// Opens the on-disk embedding cache under the scan root. Swallows
/// the I/O error with a `CoreError::Embedding` — if the cache
/// directory cannot be created the whole pass is degraded.
fn open_cache(scan_root: &Path, spec: &EmbeddingSpec) -> Result<EmbeddingCache, CoreError> {
    let base = crate::paths::cache_dir(scan_root);
    EmbeddingCache::open(&base, spec).map_err(|source| CoreError::Embedding {
        message: format!("open embedding cache: {source}"),
    })
}

/// Produces embedding vectors for fingerprints whose provider request succeeds.
fn compute_embeddings(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    corpus: &CorpusView<'_>,
    dimensions: usize,
    batch_yield: Option<Duration>,
    progress: Option<&dyn Fn(usize)>,
    observer: &mut EmbeddingObserver,
) -> EmbeddingBatch {
    let mut batch = EmbeddingBatch::with_capacity(corpus.fingerprints.len());
    let pending = lookup_phase(
        corpus,
        cache,
        &mut batch,
        observer,
        provider.max_input_chars(),
    );
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

/// Walks the corpus once, grouping fingerprints by snippet content and
/// routing each group whole: cache hit, oversized rejection, or one
/// pending provider request. `max_input_chars` is the budget the provider
/// declared for itself ([`EmbeddingProvider::max_input_chars`]) —
/// never a constant of this pass's own.
///
/// The group is what the provider request and the ANN index point are
/// both deduplicated to; its *members* never are, which is why the vector
/// travels with the whole owner list. Byte-identical clones share one
/// snippet by definition, so collapsing a group to its first member
/// deletes the embedding evidence for exactly the pairs this tool exists
/// to find, rendering `embedding_cos = 0.0` — measured-and-absent.
fn lookup_phase(
    corpus: &CorpusView<'_>,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    observer: &mut EmbeddingObserver,
    max_input_chars: usize,
) -> Vec<PendingEmbedding> {
    let mut pending: Vec<PendingEmbedding> = Vec::new();
    for group in group_snippets_by_content(corpus) {
        route_snippet_group(group, cache, batch, observer, max_input_chars, &mut pending);
    }
    pending
}

/// Fingerprints sharing one exact source snippet.
struct SnippetGroup {
    /// Every fingerprint index whose source text is `snippet`, in
    /// corpus order.
    fingerprint_indices: Vec<usize>,
    /// The shared source text.
    snippet: String,
    /// Stable content hash of `snippet`.
    snippet_hash: String,
}

/// Groups corpus fingerprints by snippet content hash, preserving
/// first-seen corpus order so downstream dispatch is deterministic.
fn group_snippets_by_content(corpus: &CorpusView<'_>) -> Vec<SnippetGroup> {
    let mut positions: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<SnippetGroup> = Vec::new();
    for (index, fingerprint) in corpus.fingerprints.iter().enumerate() {
        let snippet = snippet_for(fingerprint, corpus.sources);
        let snippet_hash = content_hash(&snippet);
        if let Some(&position) = positions.get(&snippet_hash) {
            extend_group(&mut groups, position, index);
        } else {
            let _previous = positions.insert(snippet_hash.clone(), groups.len());
            groups.push(SnippetGroup {
                fingerprint_indices: vec![index],
                snippet,
                snippet_hash,
            });
        }
    }
    groups
}

/// Appends one more owner to an existing snippet group.
fn extend_group(groups: &mut [SnippetGroup], position: usize, fingerprint_index: usize) {
    if let Some(group) = groups.get_mut(position) {
        group.fingerprint_indices.push(fingerprint_index);
    }
}

/// Routes one snippet group onto cache-hit, oversized-rejection, or
/// queue-pending. Every route applies to the whole group.
fn route_snippet_group(
    group: SnippetGroup,
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    observer: &mut EmbeddingObserver,
    max_input_chars: usize,
    pending: &mut Vec<PendingEmbedding>,
) {
    if group.snippet.chars().count() > max_input_chars {
        record_oversized_input(batch, group, max_input_chars);
        return;
    }
    observer.record_group(group.fingerprint_indices.len());
    // The cache is a deserialisation boundary: its entries are bytes
    // from disk, not values this process produced. `push_fresh_embedding`
    // guarantees nothing non-finite is ever written, so this check costs
    // one pass over a hit and can only fire on an entry that predates
    // that guarantee or was corrupted underneath us. A rejected hit
    // falls through to a fresh provider request, so the snippet is
    // re-measured rather than dropped.
    if let Some(cached) = cache
        .get(&group.snippet)
        .filter(|vector| is_finite_vector(vector))
    {
        batch.push(&group.fingerprint_indices, &cached);
        observer.record_cache_hit();
        return;
    }
    observer.record_cache_miss();
    pending.push(PendingEmbedding {
        fingerprint_indices: group.fingerprint_indices,
        snippet: group.snippet,
        snippet_hash: group.snippet_hash,
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

/// Counts an oversized snippet group as skipped before provider dispatch.
fn record_oversized_input(batch: &mut EmbeddingBatch, group: SnippetGroup, max_input_chars: usize) {
    let item = PendingEmbedding {
        fingerprint_indices: group.fingerprint_indices,
        snippet: group.snippet,
        snippet_hash: group.snippet_hash,
    };
    record_failed_pending(batch, &item, &format!("exceeds {max_input_chars} chars"));
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
        push_fresh_embedding(cache, batch, item, &vector, dimensions);
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

/// Stores one successful provider vector when it is well-formed.
///
/// A vector must clear both gates before it reaches the cache, the ANN
/// index, or a rendered signal. Dimension disagreement is the obvious
/// one; finiteness is the quiet one. A response may be valid JSON and
/// still overflow `f32` — `3.5e38` parses fine and becomes `inf`, whose
/// normalization is `NaN`, and every comparison against `NaN` is false.
/// Such a vector does not fail loudly downstream: it slips past the
/// admission floors that are written as `cosine < MIN` and manufactures
/// clusters out of malformed provider output. Rejecting at ingest is the
/// only place the vector is still attributable to the request that
/// produced it, so it is counted failed exactly like an oversized input.
fn push_fresh_embedding(
    cache: &EmbeddingCache,
    batch: &mut EmbeddingBatch,
    item: &PendingEmbedding,
    vector: &[f32],
    dimensions: usize,
) {
    if vector.len() != dimensions {
        let message = format!("expected {dimensions} dims, got {}", vector.len());
        record_failed_pending(batch, item, &message);
        return;
    }
    if !is_finite_vector(vector) {
        let message = "non-finite vector component";
        record_failed_pending(batch, item, &message);
        return;
    }
    if let Err(error) = cache.store(&item.snippet, vector) {
        tracing::warn!(%error, content_hash = %item.snippet_hash, "embedding cache write failed");
    }
    batch.push(&item.fingerprint_indices, vector);
}

/// Returns `true` when every component of `vector` is finite.
fn is_finite_vector(vector: &[f32]) -> bool {
    vector.iter().all(|value| value.is_finite())
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

/// Records one failed pending embedding request. Every fingerprint that
/// shared the rejected snippet is counted failed — anything less would
/// under-report the failure figure by exactly the duplicate count.
fn record_failed_pending<E: std::fmt::Display>(
    batch: &mut EmbeddingBatch,
    item: &PendingEmbedding,
    error: &E,
) {
    batch.failures = batch
        .failures
        .saturating_add(item.fingerprint_indices.len());
    tracing::warn!(
        error = %error,
        occurrences = item.fingerprint_indices.len(),
        snippet_chars = item.snippet.chars().count(),
        content_hash = %item.snippet_hash,
        "embedding provider rejected subtree — skipping embedding signal"
    );
}
