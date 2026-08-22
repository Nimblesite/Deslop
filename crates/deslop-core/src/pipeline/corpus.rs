//! Fingerprint-corpus assembly shared by [`super::run`] and
//! [`super::session`]. Parses discovered files through the registered
//! language parsers, honours the incremental fingerprint cache
//! ([PIPELINE-INCREMENTAL]), and returns normalised trees plus
//! fingerprints plus the per-file source bytes the embedding pass will
//! reuse.

use std::{
    collections::HashMap,
    fs,
    path::Path,
    time::Instant,
};

use crate::{
    ast::NormalizedNode,
    boilerplate::{collect_import_boilerplate_ranges, BoilerplateRange},
    discover::DiscoveredFile,
    error::CoreError,
    fingerprint::{collect_non_boilerplate_fingerprints, Fingerprint},
    fpcache::{sweep_store, CachedFile, FingerprintCache, LiveBlobs},
    lang::LanguageParser,
    report::CacheStats,
    report_metrics::{count_analysed_lines, AnalysedLines},
    sibling::collect_non_boilerplate_sibling_fingerprints,
    state::FileId,
};

use super::{config::PipelineConfig, signatures::signatures_for_file};

/// Corpus-build observability counters
/// ([PIPELINE-OBSERVABILITY-STAGES]).
mod stats;
#[cfg(test)]
mod tests;

pub use stats::CorpusBuildStats;

/// Files between corpus-build progress records
/// ([PIPELINE-OBSERVABILITY-STAGES]). Count-based so the cadence is
/// deterministic for a given file list; each record carries elapsed
/// time so throughput is readable from any two records.
const CORPUS_PROGRESS_FILE_INTERVAL: usize = 250;

/// Output of [`fingerprint_corpus`]. Kept together so the file
/// sources can be reused by the embedding pass without re-reading
/// from disk.
#[derive(Debug, Default)]
pub struct FingerprintCorpus {
    /// File contents keyed by [`FileId`] so the embedding pass can
    /// read the exact bytes referenced by a fingerprint without
    /// re-reading the file once per subtree.
    pub sources: HashMap<FileId, Vec<u8>>,
    /// Per-file cached parse + fingerprint bundle keyed by
    /// [`FileId`]. The session moves these into its canonical flat
    /// store in workspace-relative-path order
    /// ([PIPELINE-DETERMINISM]); nothing re-flattens per render.
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
    let mut build = CorpusBuildStats::default();
    let mut live_blobs = LiveBlobs::default();
    let cache_base = crate::paths::cache_dir(&config.root);
    let mut caches: HashMap<&'static str, FingerprintCache> = HashMap::new();
    let started = Instant::now();
    let mut fingerprints_running: usize = 0;
    for (position, discovered) in files.iter().enumerate() {
        let Some(parser) = parser_for_language(parsers, discovered.language) else {
            continue;
        };
        let read_started = Instant::now();
        let source = read_source(&discovered.path)?;
        build.add_read(read_started.elapsed());
        if config.incremental {
            live_blobs.record(discovered.language, &source);
        }
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
            &mut build,
        ) {
            Ok(processed) => processed,
            // A single pathologically deep file is skipped, not fatal: it
            // would otherwise overflow the recursive walks and abort the
            // whole batch run. Genuine parser errors still propagate.
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
        let lines = count_analysed_lines(&source);
        fingerprints_running = fingerprints_running.saturating_add(processed.fingerprints.len());
        let _previous_lines = corpus.analysed_lines.insert(discovered.file_id, lines);
        let _previous = corpus.per_file.insert(discovered.file_id, processed);
        let _previous_source = corpus.sources.insert(discovered.file_id, source);
        log_corpus_progress(position, files.len(), fingerprints_running, started);
    }
    log_corpus_built(files.len(), fingerprints_running, &corpus, &build, started);
    // [PIPELINE-INCREMENTAL-RETENTION] A full pass is the one moment
    // the live blob set is exactly known, so retention runs here —
    // never on a single-file change pass, and never when the store is
    // disabled (the opt-out must leave the store untouched).
    if config.incremental {
        sweep_store(&cache_base, &live_blobs, config.min_nodes);
    }
    Ok(corpus)
}

/// One fixed-interval corpus-build progress record
/// ([PIPELINE-OBSERVABILITY-STAGES]): counts and elapsed time only,
/// never paths or contents, per the logging rules.
fn log_corpus_progress(position: usize, files_total: usize, fingerprints: usize, started: Instant) {
    let files_done = position.saturating_add(1);
    if files_done % CORPUS_PROGRESS_FILE_INTERVAL != 0 {
        return;
    }
    tracing::info!(
        files_done,
        files_total,
        fingerprints,
        elapsed_ms = crate::observe::elapsed_ms(started),
        "fingerprint corpus progress"
    );
}

/// The corpus-build completion record, attributing the stage's time to
/// its substages ([PIPELINE-OBSERVABILITY-STAGES]).
fn log_corpus_built(
    files_processed: usize,
    fingerprints: usize,
    corpus: &FingerprintCorpus,
    build: &CorpusBuildStats,
    started: Instant,
) {
    tracing::info!(
        files_processed,
        fingerprints,
        cache_hits = corpus.cache_stats.hits,
        cache_misses = corpus.cache_stats.misses,
        signatures_built = build.signatures_built,
        signatures_reused = build.signatures_reused,
        exact_fingerprints = build.exact_fingerprints,
        sibling_fingerprints = build.sibling_fingerprints,
        read_ms = build.read_ms(),
        parse_ms = build.parse_ms(),
        fingerprint_ms = build.fingerprint_ms(),
        signature_ms = build.signature_ms(),
        elapsed_ms = crate::observe::elapsed_ms(started),
        "fingerprint corpus built",
    );
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
    build: &mut CorpusBuildStats,
) -> Result<(CachedFile, Vec<u8>, u64), CoreError> {
    let source = read_source(path)?;
    let cache_base = crate::paths::cache_dir(&config.root);
    let mut caches: HashMap<&'static str, FingerprintCache> = HashMap::new();
    let cache = if config.incremental {
        fingerprint_cache_for(&mut caches, &cache_base, parser.id(), config.min_nodes)
    } else {
        None
    };
    let min_nodes = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let processed = load_or_parse_file(cache, parser, &source, file_id, min_nodes, stats, build)?;
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

/// Resolves one file's normalised tree + fingerprints + signatures,
/// consulting the cache first when enabled
/// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]). Cache-miss parses the
/// file, fingerprints it, builds its signatures, and persists the
/// bundle before returning.
fn load_or_parse_file(
    cache: Option<&FingerprintCache>,
    parser: &dyn LanguageParser,
    source: &[u8],
    file_id: FileId,
    min_nodes: usize,
    stats: &mut CacheStats,
    build: &mut CorpusBuildStats,
) -> Result<CachedFile, CoreError> {
    if let Some(cache) = cache {
        if let Some(hit) = validated_cache_hit(cache, parser, source, file_id, min_nodes) {
            stats.hits = stats.hits.saturating_add(1);
            build.add_reused(hit.signatures.len());
            return Ok(hit);
        }
        stats.misses = stats.misses.saturating_add(1);
    }
    let built = build_cached_file(parser, source, file_id, min_nodes, build)?;
    build.add_built(built.signatures.len());
    persist_cached_file(cache, source, &built);
    Ok(built)
}

/// Serves a cache hit only when the stored fingerprints equal the set
/// re-derived from the cached tree under today's filters. The stored
/// signatures are positionally bound to that list, so any disagreement
/// invalidates them too — the file then falls through to the full
/// parse path and the store that follows self-heals the blob.
fn validated_cache_hit(
    cache: &FingerprintCache,
    parser: &dyn LanguageParser,
    source: &[u8],
    file_id: FileId,
    min_nodes: usize,
) -> Option<CachedFile> {
    let hit = cache.get(source, file_id)?;
    let rederived = fingerprints_for(&hit.tree, min_nodes, parser.id(), None);
    if rederived == hit.fingerprints {
        return Some(hit);
    }
    tracing::warn!(
        language = parser.id(),
        stored = hit.fingerprints.len(),
        rederived = rederived.len(),
        "cached fingerprints disagree with re-derived set — treating as miss",
    );
    None
}

/// Parses `source` and assembles the full cache bundle: normalised
/// tree, boilerplate-filtered fingerprints, and their `MinHash`
/// signatures ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]). Each substage's
/// elapsed time accumulates onto `build`
/// ([PIPELINE-OBSERVABILITY-STAGES]).
fn build_cached_file(
    parser: &dyn LanguageParser,
    source: &[u8],
    file_id: FileId,
    min_nodes: usize,
    build: &mut CorpusBuildStats,
) -> Result<CachedFile, CoreError> {
    let parse_started = Instant::now();
    let tree = parser.parse_and_normalize(source, file_id)?;
    build.add_parse(parse_started.elapsed());
    let fingerprint_started = Instant::now();
    let fingerprints = fingerprints_for(&tree, min_nodes, parser.id(), Some(build));
    build.add_fingerprint(fingerprint_started.elapsed());
    let signature_started = Instant::now();
    let signatures = signatures_for_file(&tree, &fingerprints, Some(parser.id()));
    build.add_signature(signature_started.elapsed());
    Ok(CachedFile {
        tree,
        fingerprints,
        signatures,
    })
}

/// Persists a freshly-built bundle; write failures are non-fatal —
/// the run simply stays uncached for that file.
fn persist_cached_file(cache: Option<&FingerprintCache>, source: &[u8], built: &CachedFile) {
    if let Some(cache) = cache {
        if let Err(error) = cache.store(source, built) {
            tracing::warn!(%error, "fingerprint cache write failed");
        }
    }
}

/// Collects structural and sibling fingerprints after boilerplate
/// filtering, recording the per-family split when a build accumulator
/// is supplied ([PIPELINE-OBSERVABILITY-STAGES]). The cache-validation
/// re-derivation passes `None` — it produces no new fingerprints.
fn fingerprints_for(
    normalised: &NormalizedNode,
    min_nodes: usize,
    language: &str,
    build: Option<&mut CorpusBuildStats>,
) -> Vec<Fingerprint> {
    let mut fingerprints = collect_non_boilerplate_fingerprints(normalised, min_nodes, language);
    let exact_count = fingerprints.len();
    fingerprints.extend(collect_non_boilerplate_sibling_fingerprints(
        normalised, min_nodes, language,
    ));
    if let Some(build) = build {
        build.add_fingerprint_kinds(
            exact_count,
            fingerprints.len().saturating_sub(exact_count),
        );
    }
    fingerprints
}

/// Logs that a pathologically deep file is being skipped. Shared by
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
        csharp::CSharpParser,
        dart::DartParser,
        fsharp::FSharpParser,
        go::GoParser,
        javascript::JavaScriptParser,
        php::PhpParser,
        python::PythonParser,
        rust_lang::RustParser,
        typescript::{TsxParser, TypeScriptParser},
    };
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
        Box::new(DartParser::new()),
        Box::new(JavaScriptParser::new()),
        Box::new(TypeScriptParser::new()),
        Box::new(TsxParser::new()),
        Box::new(PhpParser::new()),
        Box::new(FSharpParser::new()),
        Box::new(GoParser::new()),
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
/// the registry when a language is added ([PIPELINE-LANG-TRAIT]).
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
