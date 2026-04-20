//! Top-level pipeline orchestration.
//!
//! Glues [PIPELINE-DISCOVER-FILES], [PIPELINE-NORMALIZE-AST],
//! [PIPELINE-FINGERPRINT-MERKLE], [PIPELINE-CLUSTER-EXACT], and
//! [PIPELINE-RANK-WORST-FIRST] together behind a single entry point used by
//! the CLI and (later) the MCP/LSP daemon. Exclusion policy
//! ([EXCLUSION-CONFIG]) is applied here: `exclude` filters discovery,
//! `report_hide` flows into the renderer so hidden-only clusters are
//! omitted. Embedding policy ([FUSION-EMBED-PROVIDER]) is applied here
//! too: `EmbeddingSettings` picks a provider/mode and the pass folds
//! `embedding_cos` into candidate pairs before fusion.

use std::{collections::HashMap, fs, path::Path};

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    config::ExclusionConfig,
    discover::{discover_files, DiscoveredFile},
    embedding::{
        cache::DEFAULT_CACHE_DIR_NAME, content_hash, embedding_pairs, EmbeddingCache,
        EmbeddingMode, EmbeddingPair, EmbeddingProvider, EmbeddingSpec,
    },
    error::CoreError,
    fingerprint::{collect_fingerprints, Fingerprint},
    fpcache::{CachedFile, FingerprintCache},
    lang::{csharp::CSharpParser, python::PythonParser, rust_lang::RustParser, LanguageParser},
    lsh::{band_collisions, minhash_signature, Signature},
    pair::{candidate_pairs, cluster_by_transitive_closure},
    report::{render_report, CacheStats, EmbeddingProvenance, Report, ReportInputs},
    sibling::collect_sibling_fingerprints,
    state::FileId,
    tokens::{kgrams, token_stream_for_fingerprint, KGRAM_WIDTH},
};

/// Configuration for a single pipeline run.
#[derive(Debug)]
pub struct PipelineConfig<'a> {
    /// Root directory to analyse.
    pub root: std::path::PathBuf,
    /// Minimum AST subtree node count to consider a clone candidate
    /// ([DECISION-MIN-NODES]).
    pub min_nodes: u32,
    /// Optional explicit path to a `.codededup.toml` config. When
    /// `None`, the pipeline looks for one in `root` and falls back to
    /// the empty config when absent ([EXCLUSION-CONFIG]).
    pub config_path: Option<std::path::PathBuf>,
    /// How the embedding pass should behave.
    pub embedding: EmbeddingSettings<'a>,
    /// When true, the parse+fingerprint stage consults an on-disk
    /// cache keyed by `(language, tool_version, min_nodes,
    /// content_hash)` and rehydrates unchanged files without invoking
    /// tree-sitter ([PIPELINE-INCREMENTAL]). Missed entries are
    /// populated after a successful parse. Safe to leave enabled —
    /// key changes guarantee stale blobs can never leak into a run.
    pub incremental: bool,
}

/// Embedding policy for a pipeline run. The provider is borrowed so
/// callers can keep the trait object alive for the whole run without
/// incurring an `Arc`/`Box` per pipeline invocation.
#[derive(Debug)]
pub struct EmbeddingSettings<'a> {
    /// How aggressive to be about running the pass.
    pub mode: EmbeddingMode,
    /// The configured provider. `None` is only valid for
    /// [`EmbeddingMode::Off`] — the pipeline short-circuits before
    /// touching the trait object.
    pub provider: Option<&'a dyn EmbeddingProvider>,
}

/// Runs the full analysis pipeline and returns a rendered report.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a discovered source file cannot be read,
/// [`CoreError::ConfigParse`] / [`CoreError::ConfigPattern`] when the
/// exclusion config is malformed, and propagates any [`CoreError`] from
/// the language parser.
pub fn run(config: &PipelineConfig<'_>) -> Result<Report, CoreError> {
    let exclusion = load_exclusion_config(config)?;
    let parsers: Vec<Box<dyn LanguageParser>> = default_parsers();
    let extension_to_language = build_extension_map(&parsers);
    let discovery = discover_files(&config.root, &extension_to_language, &exclusion);

    tracing::info!(
        file_count = discovery.files.len(),
        lang_count = parsers.len(),
        config_source = %exclusion.source_path().display(),
        "file discovery complete",
    );

    let corpus = fingerprint_corpus(&discovery.files, &parsers, config)?;

    tracing::info!(
        fingerprint_count = corpus.fingerprints.len(),
        files_cache_hit = corpus.cache_stats.hits,
        files_cache_miss = corpus.cache_stats.misses,
        incremental = config.incremental,
        "fingerprinting complete",
    );

    let signatures = build_signatures(&corpus.fingerprints, &corpus.trees);
    let lsh_pairs = band_collisions(&signatures);

    let embedding_outcome = run_embedding_pass(config, &corpus)?;
    let pairs = candidate_pairs(
        &corpus.fingerprints,
        &signatures,
        &lsh_pairs,
        &embedding_outcome.pairs,
    );
    tracing::info!(
        signature_count = signatures.len(),
        lsh_pair_count = lsh_pairs.len(),
        embedding_pair_count = embedding_outcome.pairs.len(),
        candidate_pair_count = pairs.len(),
        "LSH + candidate union complete",
    );

    let fused_clusters = cluster_by_transitive_closure(&pairs);
    let clusters = build_ranked_fused_clusters(&corpus.fingerprints, &fused_clusters);
    tracing::info!(cluster_count = clusters.len(), "clustering complete");

    let file_languages = build_file_language_map(&discovery.files);
    Ok(render_report(ReportInputs {
        clusters: &clusters,
        registry: &discovery.registry,
        file_languages: &file_languages,
        files_analysed: discovery.files.len(),
        min_nodes: config.min_nodes,
        scan_root: &config.root,
        exclusion: &exclusion,
        embedding_provenance: embedding_outcome.provenance,
        cache_stats: corpus.cache_stats,
    }))
}

/// Builds a `FileId → language_id` map so the renderer can apply
/// per-language `report_hide` overlays ([EXCLUSION-CONFIG]).
fn build_file_language_map(files: &[DiscoveredFile]) -> HashMap<FileId, &'static str> {
    let mut out: HashMap<FileId, &'static str> = HashMap::new();
    for file in files {
        let _previous = out.insert(file.file_id, file.language);
    }
    out
}

/// Resolves `config.config_path` (explicit override) or falls back to
/// `DEFAULT_CONFIG_FILENAME` in the scan root.
fn load_exclusion_config(config: &PipelineConfig<'_>) -> Result<ExclusionConfig, CoreError> {
    if let Some(explicit) = &config.config_path {
        return ExclusionConfig::load(explicit);
    }
    ExclusionConfig::discover(&config.root)
}

/// Result of [`fingerprint_corpus`]. Kept together so the file
/// sources can be reused by the embedding pass without re-reading
/// from disk.
struct FingerprintCorpus {
    /// All structural + sibling fingerprints.
    fingerprints: Vec<Fingerprint>,
    /// Normalised trees kept for token extraction.
    trees: Vec<NormalizedNode>,
    /// File contents keyed by [`FileId`] so the embedding pass can
    /// read the exact bytes referenced by a fingerprint without
    /// re-reading the file once per subtree.
    sources: HashMap<FileId, Vec<u8>>,
    /// Per-run incremental-cache hit/miss counters
    /// ([PIPELINE-INCREMENTAL]).
    cache_stats: CacheStats,
}

/// Parses every discovered file and collects its structural + sibling
/// fingerprints plus the normalised tree kept for token extraction.
/// Honours [`PipelineConfig::incremental`] — on a hit, parse + normalise
/// + collect is replaced by a single cache read.
fn fingerprint_corpus(
    files: &[DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    config: &PipelineConfig<'_>,
) -> Result<FingerprintCorpus, CoreError> {
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let mut all_fingerprints: Vec<Fingerprint> = Vec::new();
    let mut all_trees: Vec<NormalizedNode> = Vec::with_capacity(files.len());
    let mut sources: HashMap<FileId, Vec<u8>> = HashMap::with_capacity(files.len());
    let mut stats = CacheStats::default();
    let cache_base = config.root.join(DEFAULT_CACHE_DIR_NAME);
    let mut caches: HashMap<&'static str, FingerprintCache> = HashMap::new();
    for discovered in files {
        let Some(parser) = parser_for_language(parsers, discovered.language) else {
            continue;
        };
        let source = read_source(&discovered.path)?;
        let cache = if config.incremental {
            fingerprint_cache_for(
                &mut caches,
                &cache_base,
                discovered.language,
                config.min_nodes,
            )
        } else {
            None
        };
        let processed = load_or_parse_file(
            cache,
            parser,
            &source,
            discovered.file_id,
            min_nodes_usize,
            &mut stats,
        )?;
        all_fingerprints.extend(processed.fingerprints);
        all_trees.push(processed.tree);
        let _previous = sources.insert(discovered.file_id, source);
    }
    Ok(FingerprintCorpus {
        fingerprints: all_fingerprints,
        trees: all_trees,
        sources,
        cache_stats: stats,
    })
}

/// Returns (lazily-opened) [`FingerprintCache`] for `language`,
/// memoised for the duration of the run. Open failures are downgraded
/// to `None` with a warning — the pipeline then runs uncached for
/// that language rather than aborting a whole run over a missing
/// cache directory.
fn fingerprint_cache_for<'a>(
    caches: &'a mut HashMap<&'static str, FingerprintCache>,
    base: &Path,
    language: &'static str,
    min_nodes: u32,
) -> Option<&'a FingerprintCache> {
    if !caches.contains_key(language) {
        match FingerprintCache::open(base, language, min_nodes) {
            Ok(cache) => {
                let _previous = caches.insert(language, cache);
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    language,
                    "fingerprint cache unavailable — falling back to full parse",
                );
                return None;
            }
        }
    }
    caches.get(language)
}

/// Resolves one file's normalised tree + fingerprints, consulting the
/// cache first when enabled. Cache-miss parses the file, fingerprints
/// it, and persists the result before returning.
fn load_or_parse_file(
    cache: Option<&FingerprintCache>,
    parser: &dyn LanguageParser,
    source: &[u8],
    file_id: FileId,
    min_nodes: usize,
    stats: &mut CacheStats,
) -> Result<CachedFile, CoreError> {
    if let Some(cache) = cache {
        if let Some(hit) = cache.get(source, file_id) {
            stats.hits = stats.hits.saturating_add(1);
            return Ok(hit);
        }
        stats.misses = stats.misses.saturating_add(1);
    }
    let normalised = parser.parse_and_normalize(source, file_id)?;
    let mut fingerprints = collect_fingerprints(&normalised, min_nodes);
    fingerprints.extend(collect_sibling_fingerprints(&normalised, min_nodes));
    let cached = CachedFile {
        tree: normalised,
        fingerprints,
    };
    if let Some(cache) = cache {
        if let Err(error) = cache.store(source, &cached) {
            tracing::warn!(%error, "fingerprint cache write failed");
        }
    }
    Ok(cached)
}

/// Computes a `MinHash` signature per fingerprint. Each signature is
/// generated from k-grams of the normalised token stream of the fingerprint's
/// subtree — token Jaccard then acts as the Type-3 recall signal per
/// [DECISION-TYPE3-TWO-PASS].
fn build_signatures(fingerprints: &[Fingerprint], trees: &[NormalizedNode]) -> Vec<Signature> {
    let mut signatures: Vec<Signature> = Vec::with_capacity(fingerprints.len());
    for fingerprint in fingerprints {
        let signature = tree_for_file(trees, fingerprint)
            .and_then(|root| token_stream_for_fingerprint(root, fingerprint))
            .map_or_else(default_signature, |tokens| signature_for_tokens(&tokens));
        signatures.push(signature);
    }
    signatures
}

/// Returns the normalised AST root for `fingerprint`'s file by scanning
/// the per-run tree list. O(n) per lookup; acceptable because the number
/// of files is small compared to the number of fingerprints.
fn tree_for_file<'a>(
    trees: &'a [NormalizedNode],
    fingerprint: &Fingerprint,
) -> Option<&'a NormalizedNode> {
    trees
        .iter()
        .find(|tree| tree.file_id == fingerprint.file_id)
}

/// Produces a signature from a prepared token stream using the configured
/// k-gram width.
fn signature_for_tokens(tokens: &[&'static str]) -> Signature {
    let grams = kgrams(tokens, KGRAM_WIDTH);
    let gram_slices: Vec<&[&'static str]> = grams.into_iter().collect();
    minhash_signature(&gram_slices)
}

/// Default signature used when no k-grams are available (subtree too
/// small to produce any). Every slot saturates at `u64::MAX`.
fn default_signature() -> Signature {
    [u64::MAX; crate::lsh::SIGNATURE_LEN]
}

/// Outcome of the embedding pass. Empty `pairs` + `None` provenance
/// means the pass was skipped or failed gracefully.
struct EmbeddingOutcome {
    /// ANN-nearest-neighbour pairs produced by the embedding pass.
    pairs: Vec<EmbeddingPair>,
    /// Provenance to record in the rendered report.
    provenance: Option<EmbeddingProvenance>,
}

/// Runs the embedding pass honouring `config.embedding.mode`:
///
/// - `Off` → skip entirely.
/// - `Auto` → try; on failure log a warning and continue with empty
///   pairs.
/// - `Required` → try; on failure propagate so the CLI exits
///   non-zero.
fn run_embedding_pass(
    config: &PipelineConfig<'_>,
    corpus: &FingerprintCorpus,
) -> Result<EmbeddingOutcome, CoreError> {
    if matches!(config.embedding.mode, EmbeddingMode::Off) {
        return Ok(EmbeddingOutcome {
            pairs: Vec::new(),
            provenance: None,
        });
    }
    match embed_corpus(config, corpus) {
        Ok(outcome) => Ok(outcome),
        Err(source) if matches!(config.embedding.mode, EmbeddingMode::Auto) => {
            tracing::warn!(error = %source, "embedding pass unavailable — continuing without Type-4 recall");
            Ok(EmbeddingOutcome {
                pairs: Vec::new(),
                provenance: None,
            })
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
        return Ok(EmbeddingOutcome {
            pairs: Vec::new(),
            provenance: None,
        });
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

/// Reads a source file into bytes.
fn read_source(path: &Path) -> Result<Vec<u8>, CoreError> {
    fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Returns the registered language parsers in a stable order
/// (implements [PIPELINE-LANG-TRAIT]).
fn default_parsers() -> Vec<Box<dyn LanguageParser>> {
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
    ]
}

/// Builds a lowercase-extension → language-id lookup from the parser
/// registry. Returning the language id (not a parser index) lets
/// [`discover_files`] check [`ExclusionConfig`] before the parser is
/// selected.
fn build_extension_map(parsers: &[Box<dyn LanguageParser>]) -> HashMap<String, &'static str> {
    let mut out: HashMap<String, &'static str> = HashMap::new();
    for parser in parsers {
        for extension in parser.file_extensions() {
            let _previous = out.insert((*extension).to_lowercase(), parser.id());
        }
    }
    out
}

/// Returns the parser whose `id()` matches `language`.
fn parser_for_language<'a>(
    parsers: &'a [Box<dyn LanguageParser>],
    language: &str,
) -> Option<&'a dyn LanguageParser> {
    parsers
        .iter()
        .find(|parser| parser.id() == language)
        .map(|boxed| &**boxed)
}
