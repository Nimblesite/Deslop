//! Fingerprint-corpus assembly shared by [`super::run`] and
//! [`super::session`]. Parses discovered files through the registered
//! language parsers, honours the incremental fingerprint cache
//! ([PIPELINE-INCREMENTAL]), and returns normalised trees plus
//! fingerprints plus the per-file source bytes the embedding pass will
//! reuse.

use std::{collections::HashMap, fs, path::Path, time::Instant};

use crate::{
    ast::NormalizedNode,
    boilerplate::BoilerplateRange,
    discover::DiscoveredFile,
    error::CoreError,
    fingerprint::{collect_non_boilerplate_fingerprints, Fingerprint},
    fpcache::{sweep_store, CachedFile, FingerprintCache},
    lang::LanguageParser,
    report::CacheStats,
    report_metrics::{count_analysed_lines, AnalysedLines},
    sibling::collect_non_boilerplate_sibling_fingerprints,
    state::FileId,
};

use super::{config::PipelineConfig, signatures::signatures_for_file};

/// The language-parser registry and its derived lookups
/// ([PIPELINE-LANG-TRAIT]).
mod registry;
/// Per-file processing and the cold-path sharded build
/// ([PERF-FLUTTER-TODO-CORPUS]).
mod shards;
/// Corpus-build observability counters
/// ([PIPELINE-OBSERVABILITY-STAGES]).
mod stats;
#[cfg(test)]
mod tests;

pub use registry::{
    build_extension_map, default_parsers, language_for_path, language_ids, parser_for_language,
    watched_source_extensions,
};
pub use stats::{CorpusBuildState, CorpusBuildStats};

use shards::{absorb_file_work, parallel_file_work, serial_file_work, AbsorbTarget, PassState};

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
    /// Every fingerprint, flat, in ascending `(path, file id)` order —
    /// the store's exact final layout, built directly by the corpus
    /// loop ([PERF-FLUTTER-TODO-MEMORY]). The historical per-file map
    /// doubled the whole record population during the store build
    /// (per-file buffers beside the flat vectors), which on a
    /// corpus-scale run peaked multi-GB above the resident set.
    pub fingerprints: Vec<Fingerprint>,
    /// One signature per fingerprint, positionally 1:1, stored as one
    /// contiguous segment **per file** in absorb order — no merge, no
    /// second copy, whatever the parallelism
    /// ([PERF-FLUTTER-TODO-MEMORY]). The normalised trees are
    /// deliberately **not** retained: their only later consumers
    /// re-materialise them from `sources`.
    pub signatures: Vec<Vec<crate::lsh::Signature>>,
    /// `(file id, fingerprint count)` per **processed** file, in the
    /// same ascending `(path, file id)` order — the store's entry list,
    /// which the session zips with the sorted discovery list.
    pub per_file: Vec<(FileId, usize)>,
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
    let workers = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    fingerprint_corpus_with_workers(files, parsers, config, workers)
}

/// [`fingerprint_corpus`] with the cold-path worker count injected —
/// the seam that lets the shard-merge parity pin hold the corpus
/// output independent of machine parallelism
/// ([PERF-FLUTTER-TODO-CORPUS], `cold_corpus_is_identical_for_any_worker_count`).
fn fingerprint_corpus_with_workers(
    files: &[DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    config: &PipelineConfig<'_>,
    workers: usize,
) -> Result<FingerprintCorpus, CoreError> {
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let mut corpus = FingerprintCorpus::default();
    let build = CorpusBuildState::default();
    let cache_base = crate::paths::cache_dir(&config.root);
    let started = Instant::now();
    let mut fingerprints_running: usize = 0;
    // [PERF-FLUTTER-TODO-MEMORY] Ascending `(path, file id)` — the
    // store's canonical order ([PIPELINE-DETERMINISM]) — computed
    // *before* parsing so each file's records append directly onto the
    // flat vectors. No per-file map, no second copy, no re-flatten.
    let mut ordered: Vec<&DiscoveredFile> = files.iter().collect();
    ordered.sort_by(|left, right| left.path.cmp(&right.path).then(left.file_id.cmp(&right.file_id)));
    // [PERF-FLUTTER-TODO-CORPUS] A cold, non-incremental build parses
    // and folds each file independently — the dominant wall cost
    // (parse + fingerprint + signature, ~80 s on the Flutter corpus) —
    // so it runs sharded over the ordered file list and the shards
    // merge back in order. Determinism is unchanged: the merge order
    // is the sorted order either way, and every per-file product is a
    // pure function of the file's bytes. Incremental builds stay
    // serial because their cache reads and writes share mutable state.
    let mut pass_state = PassState {
        build,
        ..PassState::default()
    };
    if config.incremental {
        let ordered_work =
            serial_file_work(&ordered, parsers, &cache_base, config, min_nodes_usize, &mut pass_state)?;
        for (position, work) in ordered_work.into_iter().enumerate() {
            if let Some(file_work) = work {
                absorb_file_work(
                    &mut corpus,
                    &mut fingerprints_running,
                    position,
                    files.len(),
                    file_work,
                    started,
                );
            }
        }
    } else {
        let mut shard_state = PassState::default();
        let mut target = AbsorbTarget {
            corpus: &mut corpus,
            fingerprints_running,
            files_total: files.len(),
            started,
            segment_open: false,
        };
        parallel_file_work(
            &ordered,
            parsers,
            &cache_base,
            config,
            &mut shard_state,
            &mut target,
            workers,
        )?;
        pass_state.build.absorb(&shard_state.build);
        pass_state.cache_stats.hits = pass_state
            .cache_stats
            .hits
            .saturating_add(shard_state.cache_stats.hits);
        pass_state.cache_stats.misses = pass_state
            .cache_stats
            .misses
            .saturating_add(shard_state.cache_stats.misses);
        fingerprints_running = target.fingerprints_running;
    }
    corpus.cache_stats.hits = corpus.cache_stats.hits.saturating_add(pass_state.cache_stats.hits);
    corpus.cache_stats.misses = corpus
        .cache_stats
        .misses
        .saturating_add(pass_state.cache_stats.misses);
    log_corpus_built(files.len(), fingerprints_running, &corpus, &pass_state.build, started);
    // [PIPELINE-INCREMENTAL-RETENTION] A full pass is the one moment
    // the live blob set is exactly known, so retention runs here —
    // never on a single-file change pass, and never when the store is
    // disabled (the opt-out must leave the store untouched).
    if config.incremental {
        sweep_store(&cache_base, &pass_state.blobs, config.min_nodes);
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
    build: &CorpusBuildState,
    started: Instant,
) {
    tracing::info!(
        files_processed,
        fingerprints,
        cache_hits = corpus.cache_stats.hits,
        cache_misses = corpus.cache_stats.misses,
        signatures_built = build.stats.signatures_built,
        signatures_reused = build.stats.signatures_reused,
        exact_fingerprints = build.stats.exact_fingerprints,
        sibling_fingerprints = build.stats.sibling_fingerprints,
        read_ms = build.stats.read_ms(),
        parse_ms = build.stats.parse_ms(),
        fingerprint_ms = build.stats.fingerprint_ms(),
        signature_ms = build.stats.signature_ms(),
        elapsed_ms = crate::observe::elapsed_ms(started),
        rss_mib = crate::observe::resident_mib(),
        signature_mib = corpus
            .signatures
            .iter()
            .map(std::vec::Vec::len)
            .sum::<usize>()
            .saturating_mul(1024)
            / (1024 * 1024),
        source_mib = corpus.sources.values().map(std::vec::Vec::len).sum::<usize>() / (1024 * 1024),
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
    build: &mut CorpusBuildState,
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
    build: &mut CorpusBuildState,
) -> Result<CachedFile, CoreError> {
    if let Some(cache) = cache {
        if let Some(hit) = validated_cache_hit(cache, parser, source, file_id, min_nodes) {
            stats.hits = stats.hits.saturating_add(1);
            build.stats.add_reused(hit.signatures.len());
            return Ok(hit);
        }
        stats.misses = stats.misses.saturating_add(1);
    }
    let parsed = build_cached_file(parser, source, file_id, min_nodes, build)?;
    build.stats.add_built(parsed.signatures.len());
    persist_cached_file(cache, source, &parsed);
    Ok(parsed)
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
    build: &mut CorpusBuildState,
) -> Result<CachedFile, CoreError> {
    let parse_started = Instant::now();
    let tree = parser.parse_and_normalize(source, file_id)?;
    build.stats.add_parse(parse_started.elapsed());
    let fingerprint_started = Instant::now();
    let fingerprints = fingerprints_for(&tree, min_nodes, parser.id(), Some(&mut build.stats));
    build.stats.add_fingerprint(fingerprint_started.elapsed());
    let signature_started = Instant::now();
    // [PERF-FLUTTER-TODO-CORPUS] One bottom-up fold per file
    // ([PIPELINE-SIGNATURE-FOLD]) replaces the historical per-fingerprint
    // root-resolving walk.
    let signatures = signatures_for_file(&tree, &fingerprints, Some(parser.id()));
    build.stats.add_signature(signature_started.elapsed());
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
        build.add_fingerprint_kinds(exact_count, fingerprints.len().saturating_sub(exact_count));
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

