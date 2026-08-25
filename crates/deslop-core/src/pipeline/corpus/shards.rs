//! Per-file processing and the cold-path sharded build
//! ([PERF-FLUTTER-TODO-CORPUS]): workers own disjoint ordered slices of
//! the file list and the main thread merges the shards back in order,
//! so the corpus output is identical for any worker count
//! (`cold_corpus_is_identical_for_any_worker_count`). Split from the
//! parent module, which owns corpus assembly and the cache path.

use std::{collections::HashMap, time::Instant};

use crate::{
    boilerplate::collect_import_boilerplate_ranges,
    discover::DiscoveredFile,
    error::CoreError,
    fingerprint::Fingerprint,
    fpcache::{FingerprintCache, LiveBlobs},
    lang::LanguageParser,
    report::CacheStats,
    report_metrics::count_analysed_lines,
    state::FileId,
};

use super::{
    fingerprint_cache_for, load_or_parse_file, log_corpus_progress, log_skip_too_deep,
    parser_for_language, read_source, CorpusBuildState, FingerprintCorpus,
};
use crate::pipeline::config::PipelineConfig;

/// The serial (incremental) path: cache reads and writes share mutable
/// state, so one file at a time.
pub(super) fn serial_file_work(
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

/// Everything the ordered merge folds each shard's records into.
pub(super) struct AbsorbTarget<'a> {
    /// The corpus under construction.
    pub(super) corpus: &'a mut FingerprintCorpus,
    /// Running fingerprint total for progress records.
    pub(super) fingerprints_running: usize,
    /// Files in the pass, for progress records.
    pub(super) files_total: usize,
    /// Pass start, for progress records.
    pub(super) started: Instant,
    /// Whether the corpus's last signature segment is still open for
    /// the current shard — one segment per shard, extended in place,
    /// keeps the segment count (and so every `SignatureIndex` lookup's
    /// search) tiny without any merge copy
    /// ([PERF-FLUTTER-TODO-MEMORY]).
    pub(super) segment_open: bool,
}

impl AbsorbTarget<'_> {
    /// Folds one file's products in, logging progress.
    fn absorb(&mut self, position: usize, work: FileWork) {
        self.fingerprints_running = self
            .fingerprints_running
            .saturating_add(work.fingerprints.len());
        let _previous_lines = self.corpus.analysed_lines.insert(work.file_id, work.lines);
        self.corpus
            .per_file
            .push((work.file_id, work.fingerprints.len()));
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
            fingerprints,
            signatures,
            ..
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
pub(super) fn parallel_file_work(
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
            let (shard_files, state) = handle.join().map_err(|_| CoreError::ParseFailed {
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
pub(super) struct FileWork {
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
pub(super) struct PassState {
    /// Per-language incremental caches, opened on first use.
    pub(super) caches: HashMap<&'static str, FingerprintCache>,
    /// Corpus-build counters and timers.
    pub(super) build: CorpusBuildState,
    /// Incremental cache hit/miss counters.
    pub(super) cache_stats: CacheStats,
    /// Live blob retention (incremental builds only).
    pub(super) blobs: LiveBlobs,
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
pub(super) fn absorb_file_work(
    corpus: &mut FingerprintCorpus,
    fingerprints_running: &mut usize,
    position: usize,
    files_total: usize,
    work: FileWork,
    started: Instant,
) {
    *fingerprints_running = fingerprints_running.saturating_add(work.fingerprints.len());
    let _previous_lines = corpus.analysed_lines.insert(work.file_id, work.lines);
    corpus
        .per_file
        .push((work.file_id, work.fingerprints.len()));
    corpus.fingerprints.extend(work.fingerprints);
    corpus.signatures.push(work.signatures);
    corpus.boilerplate_ranges.extend(work.boilerplate);
    let _previous_source = corpus.sources.insert(work.file_id, work.source);
    let _ = work.language;
    log_corpus_progress(position, files_total, *fingerprints_running, started);
}
