//! Fingerprint-corpus assembly shared by [`super::run`] and
//! [`super::session`]. Parses discovered files through the registered
//! language parsers, honours the incremental fingerprint cache
//! ([PIPELINE-INCREMENTAL]), and returns normalised trees plus
//! fingerprints plus the per-file source bytes the embedding pass will
//! reuse.

use std::{collections::HashMap, fs, path::Path};

use crate::{
    ast::NormalizedNode,
    boilerplate::{collect_import_boilerplate_ranges, BoilerplateRange},
    discover::DiscoveredFile,
    embedding::cache::DEFAULT_CACHE_DIR_NAME,
    error::CoreError,
    fingerprint::{collect_non_boilerplate_fingerprints, Fingerprint},
    fpcache::{CachedFile, FingerprintCache},
    lang::LanguageParser,
    report::CacheStats,
    report_metrics::{count_analysed_lines, AnalysedLines},
    sibling::collect_non_boilerplate_sibling_fingerprints,
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
    /// Per-file physical line counts, accumulated at file-read time
    /// so [METRICS-REPO] adds no extra I/O pass.
    pub analysed_lines: AnalysedLines,
    /// Import/prologue byte ranges suppressed from clone fingerprints.
    pub boilerplate_ranges: Vec<BoilerplateRange>,
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
        let processed = match load_or_parse_file(
            cache,
            parser,
            &source,
            discovered.file_id,
            min_nodes_usize,
            &mut corpus.cache_stats,
        ) {
            Ok(processed) => processed,
            // A single pathologically deep file is skipped, not fatal: it
            // would otherwise overflow the recursive walks and abort the
            // whole batch run (#168). Genuine parser errors still propagate.
            Err(CoreError::AstTooDeep { language, limit }) => {
                log_skip_too_deep(language, limit);
                continue;
            }
            Err(other) => return Err(other),
        };
        corpus
            .boilerplate_ranges
            .extend(collect_import_boilerplate_ranges(
                &processed.tree,
                discovered.language,
            ));
        corpus.fingerprints.extend(processed.fingerprints.clone());
        corpus.trees.push(processed.tree.clone());
        let lines = count_analysed_lines(&source);
        let _previous_lines = corpus.analysed_lines.insert(discovered.file_id, lines);
        let _previous = corpus.per_file.insert(discovered.file_id, processed);
        let _previous_source = corpus.sources.insert(discovered.file_id, source);
    }
    tracing::info!(
        files_processed = files.len(),
        fingerprints = corpus.fingerprints.len(),
        cache_hits = corpus.cache_stats.hits,
        cache_misses = corpus.cache_stats.misses,
        "fingerprint corpus built",
    );
    Ok(corpus)
}

/// Parses one file, consulting the incremental cache when enabled.
/// Used exclusively by [`super::session::PipelineSession`] to splice
/// an updated file into an existing corpus. The batch entry point
/// calls [`fingerprint_corpus`] instead.
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
) -> Result<(CachedFile, Vec<u8>, u64), CoreError> {
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
    let lines = count_analysed_lines(&source);
    Ok((processed, source, lines))
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
            return Ok(with_filtered_fingerprints(hit, min_nodes, parser.id()));
        }
        stats.misses = stats.misses.saturating_add(1);
    }
    let normalised = parser.parse_and_normalize(source, file_id)?;
    let fingerprints = fingerprints_for(&normalised, min_nodes, parser.id());
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

/// Replaces cached fingerprints with the current boilerplate-filtered set.
fn with_filtered_fingerprints(
    mut cached: CachedFile,
    min_nodes: usize,
    language: &str,
) -> CachedFile {
    cached.fingerprints = fingerprints_for(&cached.tree, min_nodes, language);
    cached
}

/// Collects structural and sibling fingerprints after boilerplate filtering.
fn fingerprints_for(
    normalised: &NormalizedNode,
    min_nodes: usize,
    language: &str,
) -> Vec<Fingerprint> {
    let mut fingerprints = collect_non_boilerplate_fingerprints(normalised, min_nodes, language);
    fingerprints.extend(collect_non_boilerplate_sibling_fingerprints(
        normalised, min_nodes, language,
    ));
    fingerprints
}

/// Logs that a pathologically deep file is being skipped (#168). Shared by
/// the batch corpus loop and the live session ([`super::session`]) so the
/// "skip, don't crash" decision carries one message. Logs only the language
/// id and depth limit — never a path, per the project logging rules.
pub fn log_skip_too_deep(language: &'static str, limit: usize) {
    tracing::warn!(language, limit, "skipping file: AST nests too deep");
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
    use crate::lang::{
        csharp::CSharpParser, dart::DartParser, python::PythonParser, rust_lang::RustParser,
    };
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
        Box::new(DartParser::new()),
    ]
}

/// Stable language ids of every registered parser, in registry order.
/// Single source of truth for any surface that needs the closed set of
/// supported languages — tool schemas, language filters, docs — so the list
/// can never drift from [`default_parsers`] ([PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn language_ids() -> Vec<&'static str> {
    default_parsers().iter().map(|parser| parser.id()).collect()
}

/// Detected display language id for a source path, derived from the parser
/// registry's declared extensions, or `"unknown"`. The single labeling map
/// shared by every human/agent surface (the HTML report highlighter, MCP page
/// summaries) so the detected language can never drift between them — or from
/// the registry when a language is added ([PIPELINE-LANG-TRAIT], #164).
#[must_use]
pub fn language_for_path(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return "unknown";
    };
    default_parsers()
        .iter()
        .find(|parser| {
            parser
                .file_extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .map_or("unknown", |parser| parser.id())
}

/// Source-file extensions of every registered parser, in registry order.
/// Single source of truth for any surface that filters filesystem events
/// by extension — e.g. the LSP live watcher — so the watched set can
/// never drift from [`default_parsers`] ([PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn watched_source_extensions() -> Vec<&'static str> {
    default_parsers()
        .iter()
        .flat_map(|parser| parser.file_extensions().iter().copied())
        .collect()
}

/// Builds a lowercase-extension → language-id lookup from the parser
/// registry. Returning the language id (not a parser index) lets
/// [`crate::discover::discover_files`] check [`crate::config::ExclusionConfig`]
/// before the parser is selected.
#[must_use]
pub fn build_extension_map(parsers: &[Box<dyn LanguageParser>]) -> HashMap<String, &'static str> {
    let mut out: HashMap<String, &'static str> = HashMap::new();
    for parser in parsers {
        for extension in parser.file_extensions() {
            let _previous = out.insert((*extension).to_lowercase(), parser.id());
        }
    }
    out
}
