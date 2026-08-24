//! Fingerprint-corpus assembly shared by [`super::run`] and
//! [`super::session`]. Parses discovered files through the registered
//! language parsers, honours the incremental fingerprint cache
//! ([PIPELINE-INCREMENTAL]), and returns normalised trees plus
//! fingerprints plus the per-file source bytes the embedding pass will
//! reuse.

use std::{collections::HashMap, fs, path::Path, time::Instant};

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

pub use stats::{CorpusBuildState, CorpusBuildStats};

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

/// The serial (incremental) path: cache reads and writes share mutable
/// state, so one file at a time.
fn serial_file_work(
    ordered: &[&DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    cache_base: &std::path::Path,
    config: &PipelineConfig<'_>,
    min_nodes_usize: usize,
    state: &mut PassState,
) -> Result<Vec<Option<FileWork>>, CoreError> {
    let mut work: Vec<Option<FileWork>> = Vec::with_capacity(ordered.len());
    for discovered in ordered {
        work.push(process_one_file(
            discovered,
            parsers,
            cache_base,
            config.incremental,
            config.min_nodes,
            min_nodes_usize,
            state,
        )?);
    }
    Ok(work)
}

/// The cold-path sharded build: files are independent, so workers own
/// disjoint ordered slices and stream each file's records through a
/// bounded queue to the main thread, which absorbs them in shard order
/// — deterministic, and never holding more than a few files' records
/// outside the flat vectors.
/// Everything the ordered merge folds each shard's records into.
struct AbsorbTarget<'a> {
    /// The corpus under construction.
    corpus: &'a mut FingerprintCorpus,
    /// Running fingerprint total for progress records.
    fingerprints_running: usize,
    /// Files in the pass, for progress records.
    files_total: usize,
    /// Pass start, for progress records.
    started: Instant,
    /// Whether the corpus's last signature segment is still open for
    /// the current shard — one segment per shard, extended in place,
    /// keeps the segment count (and so every `SignatureIndex` lookup's
    /// search) tiny without any merge copy
    /// ([PERF-FLUTTER-TODO-MEMORY]).
    segment_open: bool,
}

impl AbsorbTarget<'_> {
    /// Folds one file's products in, logging progress.
    fn absorb(&mut self, position: usize, work: FileWork) {
        self.fingerprints_running = self
            .fingerprints_running
            .saturating_add(work.fingerprints.len());
        let _previous_lines = self
            .corpus
            .analysed_lines
            .insert(work.file_id, work.lines);
        self.corpus.per_file.push((work.file_id, work.fingerprints.len()));
        self.corpus.boilerplate_ranges.extend(work.boilerplate);
        let _previous_source = self.corpus.sources.insert(work.file_id, work.source);
        log_corpus_progress(
            position,
            self.files_total,
            self.fingerprints_running,
            self.started,
        );
        // Fingerprints extend the flat vector; the file's signatures
        // extend the shard's open segment — moved, never copied.
        let FileWork {
            fingerprints, signatures, ..
        } = work;
        self.corpus.fingerprints.extend(fingerprints);
        if self.segment_open {
            if let Some(segment) = self.corpus.signatures.last_mut() {
                segment.extend(signatures);
                return;
            }
        }
        self.corpus.signatures.push(signatures);
        self.segment_open = true;
    }

    /// Closes the open segment: the next absorbed file opens a fresh
    /// one. Called once per shard boundary.
    fn close_segment(&mut self) {
        self.segment_open = false;
    }
}

/// The cold-path sharded build: workers own disjoint ordered slices and
/// return their per-file records; the main thread then merges the
/// shards in order into corpus vectors pre-sized to the exact total, so
/// the merge never reallocates and each shard's records exist exactly
/// once at any moment ([PERF-FLUTTER-TODO-MEMORY]).
fn parallel_file_work(
    ordered: &[&DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    cache_base: &std::path::Path,
    config: &PipelineConfig<'_>,
    state_out: &mut PassState,
    target: &mut AbsorbTarget<'_>,
    workers: usize,
) -> Result<(), CoreError> {
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let shard_size = ordered.len().div_ceil(workers.max(1)).max(1);
    let incremental = config.incremental;
    let min_nodes_config = config.min_nodes;
    let mut shards: Vec<Vec<Option<FileWork>>> = Vec::with_capacity(workers);
    let joined = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for chunk in ordered.chunks(shard_size) {
            let cache_path = cache_base;
            handles.push(scope.spawn(move || {
                let mut shard_files: Vec<Option<FileWork>> = Vec::with_capacity(chunk.len());
                // The cold path has no cache and no live-blob
                // retention, so this shard state stays empty.
                let mut state = PassState::default();
                for discovered in chunk {
                    let outcome = process_one_file(
                        discovered,
                        parsers,
                        cache_path,
                        incremental,
                        min_nodes_config,
                        min_nodes_usize,
                        &mut state,
                    );
                    match outcome {
                        Ok(one) => shard_files.push(one),
                        // A pathologically deep file is skipped, not
                        // fatal; genuine errors fail the shard.
                        Err(CoreError::AstTooDeep { language, limit }) => {
                            log_skip_too_deep(language, limit);
                            shard_files.push(None);
                        }
                        // A failed shard fails the build; its
                        // counters die with it.
                        Err(other) => return Err(other),
                    }
                }
                Ok((shard_files, state))
            }));
        }
        for handle in handles {
            // A panicked parse worker must fail the build, never
            // silently drop its files.
            let (shard_files, state) = handle
                .join()
                .map_err(|_| CoreError::ParseFailed {
                    language: "unknown",
                })??;
            shards.push(shard_files);
            state_out.build.absorb(&state.build);
            state_out.cache_stats.hits = state_out
                .cache_stats
                .hits
                .saturating_add(state.cache_stats.hits);
            state_out.cache_stats.misses = state_out
                .cache_stats
                .misses
                .saturating_add(state.cache_stats.misses);
        }
        Ok(())
    });
    joined?;
    // Exact capacity for the flat fingerprint vector from the counted
    // records; signatures are per-file segments and need no reserve.
    // The ordered merge moves each file's products in, one shard at a
    // time, freeing as it goes.
    let fingerprint_total: usize = shards
        .iter()
        .flatten()
        .map(|work| work.as_ref().map_or(0, |one| one.fingerprints.len()))
        .sum();
    target.corpus.fingerprints.reserve_exact(fingerprint_total);
    let mut position = 0_usize;
    for shard in shards {
        for work in shard {
            if let Some(one) = work {
                target.absorb(position, one);
            }
            position = position.saturating_add(1);
        }
        target.close_segment();
    }
    Ok(())
}

/// One processed file's products, before they merge into the corpus.
struct FileWork {
    /// The file's id.
    file_id: FileId,
    /// Its language id.
    language: &'static str,
    /// Its source bytes (moved into `sources` on absorb).
    source: Vec<u8>,
    /// Boilerplate-filtered fingerprints.
    fingerprints: Vec<Fingerprint>,
    /// One signature per fingerprint.
    signatures: Vec<crate::lsh::Signature>,
    /// Import/prologue boilerplate ranges.
    boilerplate: Vec<crate::boilerplate::BoilerplateRange>,
    /// Physical line count.
    lines: u64,
}

/// The mutable state a file-processing pass accumulates: build
/// counters, cache statistics, live-blob retention, and the per-language
/// cache handles. Bundled so the pass functions carry one `&mut`
/// instead of four.
#[derive(Default)]
struct PassState {
    /// Per-language incremental caches, opened on first use.
    caches: HashMap<&'static str, FingerprintCache>,
    /// Corpus-build counters and timers.
    build: CorpusBuildState,
    /// Incremental cache hit/miss counters.
    cache_stats: CacheStats,
    /// Live blob retention (incremental builds only).
    blobs: LiveBlobs,
}

/// Reads, parses, and folds one file, returning its products or `None`
/// when it has no parser registered. The incremental cache participates
/// only on incremental builds (the cold path passes `None`).
fn process_one_file(
    discovered: &DiscoveredFile,
    parsers: &[Box<dyn LanguageParser>],
    cache_base: &std::path::Path,
    incremental: bool,
    min_nodes_config: u32,
    min_nodes: usize,
    state: &mut PassState,
) -> Result<Option<FileWork>, CoreError> {
    let Some(parser) = parser_for_language(parsers, discovered.language) else {
        return Ok(None);
    };
    let read_started = Instant::now();
    let source = read_source(&discovered.path)?;
    state.build.stats.add_read(read_started.elapsed());
    if incremental {
        state.blobs.record(discovered.language, &source);
    }
    let cache = if incremental {
        Some(fingerprint_cache_for(
            &mut state.caches,
            cache_base,
            discovered.language,
            min_nodes_config,
        ))
    } else {
        None
    }
    .flatten();
    let processed = load_or_parse_file(
        cache,
        parser,
        &source,
        discovered.file_id,
        min_nodes,
        &mut state.cache_stats,
        &mut state.build,
    )?;
    let boilerplate = collect_import_boilerplate_ranges(&processed.tree, discovered.language);
    let lines = count_analysed_lines(&source);
    let crate::fpcache::CachedFile {
        tree: _tree,
        fingerprints,
        signatures,
    } = processed;
    Ok(Some(FileWork {
        file_id: discovered.file_id,
        language: discovered.language,
        source,
        fingerprints,
        signatures,
        boilerplate,
        lines,
    }))
}

/// Folds one file's products into the corpus and logs progress.
fn absorb_file_work(
    corpus: &mut FingerprintCorpus,
    fingerprints_running: &mut usize,
    position: usize,
    files_total: usize,
    work: FileWork,
    started: Instant,
) {
    *fingerprints_running = fingerprints_running.saturating_add(work.fingerprints.len());
    let _previous_lines = corpus.analysed_lines.insert(work.file_id, work.lines);
    corpus.per_file.push((work.file_id, work.fingerprints.len()));
    corpus.fingerprints.extend(work.fingerprints);
    corpus.signatures.push(work.signatures);
    corpus.boilerplate_ranges.extend(work.boilerplate);
    let _previous_source = corpus.sources.insert(work.file_id, work.source);
    let _ = work.language;
    log_corpus_progress(position, files_total, *fingerprints_running, started);
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
