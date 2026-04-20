//! Incremental pipeline session used by the daemon ([LIVE-STATE]).
//!
//! A [`PipelineSession`] keeps the last run's normalised trees,
//! fingerprints, and source bytes live in memory keyed by [`FileId`].
//! [`PipelineSession::update_files`] accepts a list of changed paths,
//! re-parses (or drops) just those files, splices the updated entries
//! into the in-memory corpus, and re-runs the deterministic-plus-
//! optional-embedding clustering pipeline. The embedding + fingerprint
//! caches on disk are shared with the batch path so warm reruns stay
//! cheap ([PIPELINE-INCREMENTAL], [FUSION-EMBED-PROVIDER]).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    config::ExclusionConfig,
    discover::{discover_files, DiscoveryResult},
    error::CoreError,
    fingerprint::Fingerprint,
    fpcache::CachedFile,
    lang::LanguageParser,
    lsh::band_collisions,
    pair::{candidate_pairs, cluster_by_transitive_closure},
    report::{render_report, CacheStats, Report, ReportInputs},
    report_metrics::AnalysedLines,
    state::{FileId, FileRegistry},
};

use super::{
    config::{EmbeddingSettings, PipelineConfig},
    corpus::{
        build_extension_map, default_parsers, fingerprint_corpus, parse_one_file,
        parser_for_language,
    },
    embedding_pass::run_embedding_pass,
    signatures::build_signatures,
};

/// A long-running analysis context. Owned by the daemon; one instance
/// per workspace root ([LIVE-LIFECYCLE]).
///
/// The session owns the [`FileRegistry`] used by its fingerprints, so
/// `FileId`s issued by a session are only meaningful within that
/// session — moving a fingerprint between sessions is never valid and
/// is structurally prevented by the `FileRegistry` invariant.
#[derive(Debug)]
pub struct PipelineSession {
    /// Workspace root pinned at [`PipelineSession::initialise`].
    root: PathBuf,
    /// Subtree-size floor used throughout the session.
    min_nodes: u32,
    /// Whether to consult the on-disk fingerprint cache.
    incremental: bool,
    /// Optional override pointing at a `.deslop.toml` outside the
    /// workspace root. `None` = discover inside `root`.
    config_path: Option<PathBuf>,
    /// Materialised registered parsers kept for the session lifetime
    /// so repeated `update_files` calls don't reload grammars.
    parsers: Vec<Box<dyn LanguageParser>>,
    /// Cached extension → language-id lookup built from `parsers`.
    extension_to_language: HashMap<String, &'static str>,
    /// Exclusion config loaded at [`PipelineSession::initialise`] and
    /// re-loaded by [`PipelineSession::reload_exclusion`] when the
    /// daemon detects a config change.
    exclusion: ExclusionConfig,
    /// File registry shared across the whole session. New files
    /// register on their first `update_files` sighting; removed files
    /// keep their [`FileId`] slot (nothing ever gets unregistered) so
    /// old diagnostics retain stable handles.
    registry: FileRegistry,
    /// Per-`FileId` cached tree + fingerprints. Keys here are the
    /// single source of truth for "which files are currently part of
    /// the corpus."
    per_file: HashMap<FileId, CachedFile>,
    /// Per-`FileId` source bytes so the embedding pass can read the
    /// exact snippet covered by a fingerprint without re-reading
    /// from disk.
    sources: HashMap<FileId, Vec<u8>>,
    /// Per-`FileId` absolute path. Kept separately from the registry
    /// because the registry is append-only — we also need to know
    /// which ids are *currently* part of the corpus.
    live_paths: HashMap<FileId, PathBuf>,
    /// Per-`FileId` language id, mirrored into render inputs.
    file_languages: HashMap<FileId, &'static str>,
    /// Running cache-hit telemetry. Accumulates across updates so
    /// subscribers can track long-term cache utility.
    cumulative_stats: CacheStats,
    /// Per-file analysed-line counts. Updated in place on each
    /// [`Self::update_files`] call so [METRICS-REPO] never re-reads
    /// sources from disk.
    analysed_lines: AnalysedLines,
    /// Files analysed in the most recent generation. Pre-computed so
    /// the render inputs stay cheap.
    files_analysed: usize,
}

impl PipelineSession {
    /// Runs the first full analysis against `root` and returns the
    /// session plus the initial [`Report`]. The session is then ready
    /// to accept [`Self::update_files`] calls.
    ///
    /// # Errors
    ///
    /// Propagates every error variant the batch [`super::run::run`]
    /// would produce: [`CoreError::Io`] on unreadable sources,
    /// [`CoreError::ConfigParse`] / [`CoreError::ConfigPattern`] for
    /// malformed exclusion configs, and parser errors from
    /// [`crate::lang::LanguageParser::parse_and_normalize`].
    pub fn initialise(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding: EmbeddingSettings<'_>,
    ) -> Result<(Self, Report), CoreError> {
        let parsers = default_parsers();
        let extension_to_language = build_extension_map(&parsers);
        tracing::info!(
            root = %root.display(),
            root_exists = root.exists(),
            root_is_dir = root.is_dir(),
            min_nodes,
            incremental,
            supported_extensions = ?extension_to_language.keys().collect::<Vec<_>>(),
            "pipeline session initialising",
        );
        let exclusion = load_exclusion(&root, config_path.as_deref())?;
        let discovery = discover_files(&root, &extension_to_language, &exclusion);
        log_discovery_summary(&discovery, &root);
        let config = PipelineConfig {
            root: root.clone(),
            min_nodes,
            config_path: config_path.clone(),
            embedding,
            incremental,
        };
        let corpus = fingerprint_corpus(&discovery.files, &parsers, &config)?;
        let mut live_paths: HashMap<FileId, PathBuf> = HashMap::new();
        let mut file_languages: HashMap<FileId, &'static str> = HashMap::new();
        for discovered in &discovery.files {
            let _prev = live_paths.insert(discovered.file_id, discovered.path.clone());
            let _prev_language = file_languages.insert(discovered.file_id, discovered.language);
        }
        let files_analysed = discovery.files.len();
        let mut session = Self {
            root,
            min_nodes,
            incremental,
            config_path,
            parsers,
            extension_to_language,
            exclusion,
            registry: discovery.registry,
            per_file: corpus.per_file,
            sources: corpus.sources,
            live_paths,
            file_languages,
            cumulative_stats: corpus.cache_stats,
            analysed_lines: corpus.analysed_lines,
            files_analysed,
        };
        let report = session.render(&config, corpus.cache_stats)?;
        Ok((session, report))
    }

    /// Re-parses (or drops) each path in `changed` and returns the
    /// refreshed [`Report`]. A `changed` entry that no longer exists
    /// on disk is treated as a deletion. Paths outside the workspace
    /// root or with unsupported extensions are silently skipped — a
    /// watcher can fire on any file, not only interesting ones.
    ///
    /// # Errors
    ///
    /// Same error surface as [`Self::initialise`].
    pub fn update_files(
        &mut self,
        changed: &[PathBuf],
        embedding: EmbeddingSettings<'_>,
    ) -> Result<Report, CoreError> {
        let mut stats = CacheStats::default();
        for path in changed {
            self.apply_one_change(path, &mut stats, &embedding)?;
        }
        self.cumulative_stats.hits = self.cumulative_stats.hits.saturating_add(stats.hits);
        self.cumulative_stats.misses = self.cumulative_stats.misses.saturating_add(stats.misses);
        let config = self.pipeline_config(embedding);
        self.render(&config, stats)
    }

    /// Returns the workspace root this session is analysing.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the active subtree-size floor.
    #[must_use]
    pub const fn min_nodes(&self) -> u32 {
        self.min_nodes
    }

    /// Returns the path associated with `file_id`, if the session has
    /// seen it.
    #[must_use]
    pub fn path_for(&self, file_id: FileId) -> Option<&Path> {
        self.live_paths.get(&file_id).map(PathBuf::as_path)
    }

    /// Resolves an absolute or workspace-relative path to a live
    /// [`FileId`]. Returns `None` when the path is not currently part
    /// of the corpus (e.g. the file was deleted or never matched an
    /// extension).
    #[must_use]
    pub fn file_id_for(&self, path: &Path) -> Option<FileId> {
        let target = self.canonicalise_reference(path);
        self.live_paths
            .iter()
            .find(|(_, registered)| **registered == target)
            .map(|(id, _)| *id)
    }

    /// Returns a reference to the session's file registry. Exposed so
    /// daemon-layer consumers (query API, notifications) can resolve
    /// [`FileId`]s without cloning the map.
    #[must_use]
    pub fn registry(&self) -> &FileRegistry {
        &self.registry
    }

    /// Returns the per-file language map in the same shape the
    /// renderer consumes.
    #[must_use]
    pub fn file_languages(&self) -> &HashMap<FileId, &'static str> {
        &self.file_languages
    }

    /// Returns the language parsers registered with this session.
    #[must_use]
    pub fn parsers(&self) -> &[Box<dyn LanguageParser>] {
        &self.parsers
    }

    /// Returns the currently-loaded exclusion config.
    #[must_use]
    pub const fn exclusion(&self) -> &ExclusionConfig {
        &self.exclusion
    }

    /// Returns the cumulative cache-hit telemetry since session start.
    #[must_use]
    pub const fn cumulative_cache_stats(&self) -> CacheStats {
        self.cumulative_stats
    }

    /// Returns the number of files in the corpus at the most recent
    /// generation.
    #[must_use]
    pub const fn files_analysed(&self) -> usize {
        self.files_analysed
    }

    /// Reloads the exclusion config from disk. Called by the daemon
    /// when `.deslop.toml` itself changes.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ConfigParse`] or [`CoreError::ConfigPattern`]
    /// when the new config is malformed. The session keeps the old
    /// config on failure so a bad edit does not brick the daemon.
    pub fn reload_exclusion(&mut self) -> Result<(), CoreError> {
        self.exclusion = load_exclusion(&self.root, self.config_path.as_deref())?;
        Ok(())
    }

    /// Applies one changed path: delete, update, or add.
    fn apply_one_change(
        &mut self,
        path: &Path,
        stats: &mut CacheStats,
        embedding: &EmbeddingSettings<'_>,
    ) -> Result<(), CoreError> {
        let absolute = self.canonicalise_reference(path);
        if !absolute.exists() {
            self.drop_path(&absolute);
            return Ok(());
        }
        let Some(language) = self.language_for(&absolute) else {
            return Ok(());
        };
        if self.exclusion.is_excluded(&absolute, Some(language)) {
            self.drop_path(&absolute);
            return Ok(());
        }
        let Some(parser) = parser_for_language(&self.parsers, language) else {
            return Ok(());
        };
        let file_id = self
            .file_id_for(&absolute)
            .unwrap_or_else(|| self.registry.register(absolute.clone()));
        let config = self.pipeline_config_with_mode(embedding);
        let (cached, source, lines) = parse_one_file(file_id, &absolute, parser, &config, stats)?;
        let _prev_lines = self.analysed_lines.insert(file_id, lines);
        let _prev = self.per_file.insert(file_id, cached);
        let _prev_source = self.sources.insert(file_id, source);
        let _prev_path = self.live_paths.insert(file_id, absolute);
        let _prev_lang = self.file_languages.insert(file_id, language);
        self.files_analysed = self.live_paths.len();
        Ok(())
    }

    /// Removes a path from every in-memory map if present.
    fn drop_path(&mut self, absolute: &Path) {
        let Some((file_id, _)) = self
            .live_paths
            .iter()
            .find(|(_, registered)| registered.as_path() == absolute)
            .map(|(id, path)| (*id, path.clone()))
        else {
            return;
        };
        let _removed_path = self.live_paths.remove(&file_id);
        let _removed_cache = self.per_file.remove(&file_id);
        let _removed_source = self.sources.remove(&file_id);
        let _removed_lang = self.file_languages.remove(&file_id);
        let _removed_lines = self.analysed_lines.remove(&file_id);
        self.files_analysed = self.live_paths.len();
    }

    /// Returns the registered language id that claims `path`, if any.
    fn language_for(&self, path: &Path) -> Option<&'static str> {
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_lowercase)?;
        self.extension_to_language.get(&extension).copied()
    }

    /// Resolves `path` against the workspace root so relative paths
    /// from a watcher are handled identically to absolute paths.
    fn canonicalise_reference(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    /// Builds a [`PipelineConfig`] snapshot for a pass that does not
    /// need the embedding provider — the parse-only cache-consulting
    /// path in [`parse_one_file`].
    fn pipeline_config_with_mode<'a>(
        &self,
        embedding: &EmbeddingSettings<'a>,
    ) -> PipelineConfig<'a> {
        PipelineConfig {
            root: self.root.clone(),
            min_nodes: self.min_nodes,
            config_path: self.config_path.clone(),
            embedding: EmbeddingSettings {
                mode: embedding.mode,
                provider: embedding.provider,
            },
            incremental: self.incremental,
        }
    }

    /// Builds a [`PipelineConfig`] that owns the provider reference
    /// and is suitable for [`run_embedding_pass`].
    fn pipeline_config<'a>(&self, embedding: EmbeddingSettings<'a>) -> PipelineConfig<'a> {
        PipelineConfig {
            root: self.root.clone(),
            min_nodes: self.min_nodes,
            config_path: self.config_path.clone(),
            embedding,
            incremental: self.incremental,
        }
    }

    /// Runs clustering + ranking + rendering over the current
    /// in-memory corpus. Returns a freshly rendered [`Report`].
    fn render(
        &mut self,
        config: &PipelineConfig<'_>,
        last_pass_stats: CacheStats,
    ) -> Result<Report, CoreError> {
        let corpus = self.snapshot_corpus();
        let signatures = build_signatures(&corpus.fingerprints, &corpus.trees);
        let lsh_pairs = band_collisions(&signatures);
        let embedding_outcome = run_embedding_pass(config, &corpus)?;
        let pairs = candidate_pairs(
            &corpus.fingerprints,
            &signatures,
            &lsh_pairs,
            &embedding_outcome.pairs,
        );
        let fused_clusters = cluster_by_transitive_closure(&pairs);
        let clusters = build_ranked_fused_clusters(&corpus.fingerprints, &fused_clusters);
        Ok(render_report(ReportInputs {
            clusters: &clusters,
            registry: &self.registry,
            file_languages: &self.file_languages,
            files_analysed: self.files_analysed,
            min_nodes: self.min_nodes,
            scan_root: &self.root,
            exclusion: &self.exclusion,
            embedding_provenance: embedding_outcome.provenance,
            cache_stats: last_pass_stats,
            sources: &corpus.sources,
            analysed_lines: &self.analysed_lines,
        }))
    }

    /// Flattens the per-file state into a [`super::corpus::FingerprintCorpus`]
    /// suitable for the downstream LSH / embedding / clustering stages.
    /// `per_file` is left empty because the session already owns the
    /// authoritative map — the snapshot is consumed transiently.
    fn snapshot_corpus(&self) -> super::corpus::FingerprintCorpus {
        let mut fingerprints: Vec<Fingerprint> = Vec::new();
        let mut trees: Vec<NormalizedNode> = Vec::with_capacity(self.per_file.len());
        for cached in self.per_file.values() {
            fingerprints.extend(cached.fingerprints.clone());
            trees.push(cached.tree.clone());
        }
        super::corpus::FingerprintCorpus {
            fingerprints,
            trees,
            sources: self.sources.clone(),
            per_file: HashMap::new(),
            cache_stats: CacheStats::default(),
            analysed_lines: AnalysedLines::new(),
        }
    }
}

/// Resolves the exclusion config using the session's override path or
/// falling back to the workspace default.
fn load_exclusion(root: &Path, override_path: Option<&Path>) -> Result<ExclusionConfig, CoreError> {
    if let Some(explicit) = override_path {
        return ExclusionConfig::load(explicit);
    }
    ExclusionConfig::discover(root)
}

/// Emits an info-level summary of what file discovery found, grouped by
/// language. When the count is zero, also logs a warning — this is the
/// most common "why is the report empty?" failure mode.
fn log_discovery_summary(discovery: &DiscoveryResult, root: &Path) {
    let total = discovery.files.len();
    let mut by_language: HashMap<&'static str, usize> = HashMap::new();
    for file in &discovery.files {
        let entry = by_language.entry(file.language).or_insert(0);
        *entry = entry.saturating_add(1);
    }
    tracing::info!(
        root = %root.display(),
        total_files = total,
        by_language = ?by_language,
        "file discovery complete",
    );
    if total == 0 {
        tracing::warn!(
            root = %root.display(),
            root_exists = root.exists(),
            "no source files discovered — check workspace root and language support",
        );
    }
}
