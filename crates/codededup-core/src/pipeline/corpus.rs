//! Fingerprint-corpus assembly shared by [`super::run`] and
//! [`super::session`]. Parses discovered files through the registered
//! language parsers, honours the incremental fingerprint cache
//! ([PIPELINE-INCREMENTAL]), and returns normalised trees plus
//! fingerprints plus the per-file source bytes the embedding pass will
//! reuse.

use std::{collections::HashMap, fs, path::Path};

use crate::{
    ast::NormalizedNode,
    discover::DiscoveredFile,
    embedding::cache::DEFAULT_CACHE_DIR_NAME,
    error::CoreError,
    fingerprint::{collect_fingerprints, Fingerprint},
    fpcache::{CachedFile, FingerprintCache},
    lang::LanguageParser,
    report::CacheStats,
    sibling::collect_sibling_fingerprints,
    state::FileId,
};

use super::config::PipelineConfig;

/// Output of [`fingerprint_corpus`]. Kept together so the file
/// sources can be reused by the embedding pass without re-reading
/// from disk.
#[derive(Debug, Default)]
pub struct FingerprintCorpus {
    /// All structural + sibling fingerprints.
    pub fingerprints: Vec<Fingerprint>,
    /// Normalised trees kept for token extraction.
    pub trees: Vec<NormalizedNode>,
    /// File contents keyed by [`FileId`] so the embedding pass can
    /// read the exact bytes referenced by a fingerprint without
    /// re-reading the file once per subtree.
    pub sources: HashMap<FileId, Vec<u8>>,
    /// Per-file cached parse + fingerprint bundle keyed by
    /// [`FileId`]. Used by the session to splice in updated entries
    /// without re-scanning the whole workspace.
    pub per_file: HashMap<FileId, CachedFile>,
    /// Per-run incremental-cache hit/miss counters
    /// ([PIPELINE-INCREMENTAL]).
    pub cache_stats: CacheStats,
}

/// Parses every discovered file and collects its structural + sibling
/// fingerprints plus the normalised tree kept for token extraction.
/// Honours [`PipelineConfig::incremental`] — on a hit, parse + normalise
/// + collect is replaced by a single cache read.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a source file cannot be read and
/// forwards any parser error surfaced by
/// [`crate::lang::LanguageParser::parse_and_normalize`].
pub fn fingerprint_corpus(
    files: &[DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    config: &PipelineConfig<'_>,
) -> Result<FingerprintCorpus, CoreError> {
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let mut corpus = FingerprintCorpus::default();
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
            &mut corpus.cache_stats,
        )?;
        corpus.fingerprints.extend(processed.fingerprints.clone());
        corpus.trees.push(processed.tree.clone());
        let _previous = corpus.per_file.insert(discovered.file_id, processed);
        let _previous_source = corpus.sources.insert(discovered.file_id, source);
    }
    Ok(corpus)
}

/// Parses one file, consulting the incremental cache when enabled.
/// Exposed so the session can splice an updated file into an existing
/// corpus without re-running the full [`fingerprint_corpus`] walk.
///
/// # Errors
///
/// Propagates [`CoreError::Io`] and parser errors from
/// [`crate::lang::LanguageParser::parse_and_normalize`].
pub fn parse_one_file(
    file_id: FileId,
    path: &Path,
    parser: &dyn LanguageParser,
    config: &PipelineConfig<'_>,
    stats: &mut CacheStats,
) -> Result<(CachedFile, Vec<u8>), CoreError> {
    let source = read_source(path)?;
    let cache_base = config.root.join(DEFAULT_CACHE_DIR_NAME);
    let mut caches: HashMap<&'static str, FingerprintCache> = HashMap::new();
    let cache = if config.incremental {
        fingerprint_cache_for(&mut caches, &cache_base, parser.id(), config.min_nodes)
    } else {
        None
    };
    let min_nodes = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let processed = load_or_parse_file(cache, parser, &source, file_id, min_nodes, stats)?;
    Ok((processed, source))
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

/// Reads a source file into bytes.
pub fn read_source(path: &Path) -> Result<Vec<u8>, CoreError> {
    fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Returns the parser whose `id()` matches `language`.
pub fn parser_for_language<'a>(
    parsers: &'a [Box<dyn LanguageParser>],
    language: &str,
) -> Option<&'a dyn LanguageParser> {
    parsers
        .iter()
        .find(|parser| parser.id() == language)
        .map(|boxed| &**boxed)
}

/// Returns the registered language parsers in a stable order
/// (implements [PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn default_parsers() -> Vec<Box<dyn LanguageParser>> {
    use crate::lang::{csharp::CSharpParser, python::PythonParser, rust_lang::RustParser};
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
    ]
}

/// Builds a lowercase-extension → language-id lookup from the parser
/// registry. Returning the language id (not a parser index) lets
/// [`crate::discover::discover_files`] check [`crate::config::ExclusionConfig`]
/// before the parser is selected.
#[must_use]
pub fn build_extension_map(
    parsers: &[Box<dyn LanguageParser>],
) -> HashMap<String, &'static str> {
    let mut out: HashMap<String, &'static str> = HashMap::new();
    for parser in parsers {
        for extension in parser.file_extensions() {
            let _previous = out.insert((*extension).to_lowercase(), parser.id());
        }
    }
    out
}
