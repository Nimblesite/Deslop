//! Embedding pass orchestration shared by [`super::run`] and
//! [`super::session`]. Reads the corpus, dispatches the
//! [`EmbeddingProvider`] based on the run's [`EmbeddingMode`], and
//! returns the ANN-nearest-neighbour pairs plus report provenance.

use std::{collections::HashMap, path::Path};

use crate::{
    embedding::{
        cache::DEFAULT_CACHE_DIR_NAME, content_hash, embedding_pairs, EmbeddingCache,
        EmbeddingMode, EmbeddingPair, EmbeddingProvider, EmbeddingSpec,
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
    let embeddings = compute_embeddings(provider, &cache, corpus)?;
    let pairs = embedding_pairs(&corpus.fingerprints, &embeddings);
    tracing::info!(pair_count = pairs.len(), "embedding pass complete");
    Ok(EmbeddingOutcome {
        pairs,
        provenance: Some(provenance_from(spec)),
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

/// Produces an embedding vector per fingerprint. Cache hits short-
/// circuit the provider call; misses invoke the provider and persist
/// the result for subsequent runs. Returns a vector aligned with
/// `corpus.fingerprints` — entry `i` embeds fingerprint `i`.
fn compute_embeddings(
    provider: &dyn EmbeddingProvider,
    cache: &EmbeddingCache,
    corpus: &FingerprintCorpus,
) -> Result<Vec<Vec<f32>>, CoreError> {
    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(corpus.fingerprints.len());
    for fingerprint in &corpus.fingerprints {
        let snippet = snippet_for(fingerprint, &corpus.sources);
        if let Some(cached) = cache.get(&snippet) {
            embeddings.push(cached);
            continue;
        }
        let fresh = provider
            .embed(&snippet)
            .map_err(|source| CoreError::Embedding {
                message: source.to_string(),
            })?;
        if let Err(error) = cache.store(&snippet, &fresh) {
            tracing::warn!(%error, content_hash = %content_hash(&snippet), "embedding cache write failed");
        }
        embeddings.push(fresh);
    }
    Ok(embeddings)
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
fn provenance_from(spec: EmbeddingSpec) -> EmbeddingProvenance {
    EmbeddingProvenance {
        provider_id: spec.provider_id,
        model_id: spec.model_id,
        model_version: spec.model_version,
        dimensions: spec.dimensions,
    }
}
