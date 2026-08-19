//! Incremental pipeline session used by the daemon ([LIVE-STATE]).
//!
//! A [`PipelineSession`] keeps the last run's normalised trees,
//! fingerprints, and source bytes live in memory keyed by [`FileId`].
//! [`PipelineSession::update_files`] accepts a list of changed paths,
//! re-parses (or drops) just those files, splices the updated entries
//! into the in-memory corpus, and re-runs the deterministic-plus-
//! optional-embedding clustering pipeline. The embedding + fingerprint
//! caches on disk are shared with the batch path ([PIPELINE-INCREMENTAL]).

mod ast_access;
mod change;
mod diff;
mod render;
mod store;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    boilerplate::BoilerplateRange,
    config::{is_config_path, watched_config_paths, ExclusionConfig},
    discover::{
        discover_files, is_ignore_rule_path, DiscoveredFile, DiscoveryResult, IgnoreMatcher,
    },
    error::CoreError,
    lang::LanguageParser,
    report::{CacheStats, Report},
    report_metrics::AnalysedLines,
    state::{FileId, FileRegistry},
};

use super::{
    config::{EmbeddingSettings, PipelineConfig},
    corpus::{build_extension_map, default_parsers, fingerprint_corpus, FingerprintCorpus},
};

use change::CorpusEffect;
use store::CorpusStore;

/// A long-running analysis context owned by the daemon ([LIVE-LIFECYCLE]).
///
/// The session owns the [`FileRegistry`] used by its fingerprints, so
/// `FileId`s issued by a session are only meaningful within that
/// session — moving a fingerprint between sessions is never valid and
/// is structurally prevented by the `FileRegistry` invariant.
#[derive(Debug)]
pub struct PipelineSession {
    /// Workspace root pinned at [`PipelineSession::initialise`].
    pub(super) root: PathBuf,
    /// Subtree-size floor used throughout the session.
    pub(super) min_nodes: u32,
    /// Whether the invocation requested the on-disk fingerprint cache.
    /// Gated per pass by the config escape hatch through
    /// [`Self::effective_incremental`] ([CONFIG-INCREMENTAL-OPTOUT]).
    pub(super) incremental: bool,
    /// Optional override pointing at a `.deslop.toml` outside the
    /// workspace root. `None` = discover inside `root`.
    pub(super) config_path: Option<PathBuf>,
    /// Materialised registered parsers kept for the session lifetime
    /// so repeated `update_files` calls don't reload grammars.
    pub(super) parsers: Vec<Box<dyn LanguageParser>>,
    /// Cached extension → language-id lookup built from `parsers`.
    pub(super) extension_to_language: HashMap<String, &'static str>,
    /// Exclusion config loaded at [`PipelineSession::initialise`] and
    /// re-loaded by [`PipelineSession::reload_exclusion`] when the
    /// daemon detects a config change.
    pub(super) exclusion: Arc<ExclusionConfig>,
    /// Ignore rules (`.gitignore`, `.ignore`, `.git/info/exclude`, hidden
    /// components) mirroring the ones [`discover_files`] gets from its
    /// walker. The live ingest path is handed individual paths and never
    /// walks, so it must apply these itself or it admits files discovery
    /// would have pruned. Rebuilt alongside `exclusion`.
    pub(super) ignore_matcher: IgnoreMatcher,
    /// File registry shared across the whole session. New files
    /// register on their first `update_files` sighting; removed files
    /// keep their [`FileId`] slot (nothing ever gets unregistered) so
    /// old diagnostics retain stable handles.
    pub(super) registry: FileRegistry,
    /// Canonical flat corpus storage: every fingerprint, signature,
    /// and normalised tree, in workspace-relative-path order with one
    /// span per live file. Entry presence is the single source of
    /// truth for "which files currently contribute fingerprints." A
    /// render pass borrows it as-is; only a live change copies —
    /// splicing exactly one file's records
    /// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]). Private: session
    /// submodules reach it as descendants; nothing outside does.
    store: CorpusStore,
    /// Per-`FileId` source bytes so the embedding pass can read the
    /// exact snippet covered by a fingerprint without re-reading
    /// from disk.
    pub(super) sources: HashMap<FileId, Vec<u8>>,
    /// Per-`FileId` absolute path. Kept separately from the registry
    /// because the registry is append-only — we also need to know
    /// which ids are *currently* part of the corpus.
    pub(super) live_paths: HashMap<FileId, PathBuf>,
    /// Per-`FileId` language id, mirrored into render inputs.
    pub(super) file_languages: HashMap<FileId, &'static str>,
    /// Running cache-hit telemetry. Accumulates across updates so
    /// subscribers can track long-term cache utility.
    pub(super) cumulative_stats: CacheStats,
    /// Per-file analysed-line counts. Updated in place on each
    /// [`Self::update_files`] call so [METRICS-REPO] never re-reads
    /// sources from disk.
    pub(super) analysed_lines: AnalysedLines,
    /// Import/prologue ranges suppressed from clone ranking.
    pub(super) boilerplate_ranges: Vec<BoilerplateRange>,
    /// Files analysed in the most recent generation.
    pub(super) files_analysed: usize,
    /// Verified diff scope when the session was initialised with a
    /// diff ([CLI-ARG-DIFF]). Every render tags against it.
    pub(super) diff_scope: Option<crate::diff_scope::DiffScope>,
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
        Self::initialise_with_diff(root, min_nodes, incremental, config_path, embedding, None)
    }

    /// [`Self::initialise`] with an optional parsed unified diff
    /// ([CLI-ARG-DIFF]): the diff is byte-verified against the freshly
    /// analysed corpus and, when clean, tags the initial report and
    /// every later render ([OUTPUT-SCHEMA-DIFF-TAGS]).
    ///
    /// # Errors
    ///
    /// Everything [`Self::initialise`] produces, plus
    /// [`CoreError::DiffStale`] when the diff does not match the
    /// scanned tree.
    pub fn initialise_with_diff(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding: EmbeddingSettings<'_>,
        diff: Option<&crate::diff_scope::ParsedDiff>,
    ) -> Result<(Self, Report), CoreError> {
        let parsers = default_parsers();
        let extension_to_language = build_extension_map(&parsers);
        // [#141 MCP-SAFETY] Canonicalise the root so the registry,
        // exclusion matchers and watcher inputs all share one
        // filesystem identity. Without this the macOS `/var/...` →
        // `/private/var/...` symlink pair leaves the registry holding
        // one form while the canonical root sits on the other, and
        // every later path comparison silently misses.
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        tracing::info!(
            root = %root.display(),
            root_exists = root.exists(),
            root_is_dir = root.is_dir(),
            min_nodes,
            incremental,
            supported_extensions = ?extension_to_language.keys().collect::<Vec<_>>(),
            "pipeline session initialising",
        );
        let exclusion = Arc::new(load_exclusion(&root, config_path.as_deref())?);
        // [CONFIG-INCREMENTAL-OPTOUT] The config file is the outermost
        // escape hatch: `[analysis] incremental = false` disables
        // persisted processing for every surface that reaches this
        // point — CLI batch, rerun, LSP, MCP — whatever the invocation
        // requested.
        let effective_incremental = incremental && exclusion.incremental_enabled();
        if incremental && !effective_incremental {
            tracing::info!(
                "persisted processing disabled by config (`[analysis] incremental = false`)",
            );
        }
        let ignore_matcher = IgnoreMatcher::build(&root);
        let discovery = discover_files(&root, &extension_to_language, &exclusion);
        log_discovery_summary(&discovery, &root);
        let config = PipelineConfig {
            root: root.clone(),
            min_nodes,
            config_path: config_path.clone(),
            embedding,
            incremental: effective_incremental,
        };
        let mut corpus = fingerprint_corpus(&discovery.files, &parsers, &config)?;
        let store = build_store(&mut corpus, &discovery.files, &root);
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
            ignore_matcher,
            registry: discovery.registry,
            store,
            sources: corpus.sources,
            live_paths,
            file_languages,
            cumulative_stats: corpus.cache_stats,
            analysed_lines: corpus.analysed_lines,
            boilerplate_ranges: corpus.boilerplate_ranges,
            files_analysed,
            diff_scope: None,
        };
        if let Some(parsed) = diff {
            session.attach_diff(parsed)?;
        }
        let report = session.render(&config, corpus.cache_stats)?;
        Ok((session, report))
    }

    /// Re-parses (or drops) each path in `changed` and returns the
    /// refreshed [`Report`]. A `changed` entry that no longer exists
    /// on disk is treated as a deletion. Paths outside the workspace
    /// root or with unsupported extensions are silently skipped — a
    /// watcher can fire on any file, not only interesting ones.
    ///
    /// A `changed` entry naming a watched config file
    /// (`<root>/.deslop.toml` or the explicit override) or an
    /// ignore-rule file (`.gitignore` / `.ignore` /
    /// `.git/info/exclude`) reloads the exclusion config and ignore
    /// matcher and re-evaluates the whole corpus: newly-excluded files
    /// are dropped, newly re-included files are re-discovered
    /// ([LIVE-CONFIG-LIVE] #189, ignore-rule parity #287).
    ///
    /// [LIVE-SCHEDULER-NOOP] Returns [`None`] when the pass touched no
    /// analysed file — every path was rejected by the extension,
    /// exclusion, or ignore gate, or named a file the corpus never held.
    /// The corpus is then provably unchanged, so the report is too, and
    /// re-deriving it is pure waste: one production LSP burned 11h17m of
    /// CPU across 1086 such passes before this early-out existed.
    /// Callers keep the report they already have.
    ///
    /// # Errors
    ///
    /// Same error surface as [`Self::initialise`].
    pub fn update_files(
        &mut self,
        changed: &[PathBuf],
        embedding: EmbeddingSettings<'_>,
    ) -> Result<Option<Report>, CoreError> {
        let mut stats = CacheStats::default();
        let watched = watched_config_paths(&self.root, self.config_path.as_deref());
        // A config or ignore-rule edit re-scopes rendering itself (hide
        // patterns, thresholds), not just the file set, so it always
        // re-renders even when no file enters or leaves the corpus.
        let mut effect =
            CorpusEffect::from_touched(changed.iter().any(|path| reshapes_corpus(path, &watched)));
        if effect == CorpusEffect::Mutated {
            self.refresh_exclusion(&mut stats, &embedding)?;
        }
        for path in changed
            .iter()
            .filter(|path| !reshapes_corpus(path, &watched))
        {
            effect = effect.merge(self.apply_one_change(path, &mut stats, &embedding)?);
        }
        if effect == CorpusEffect::Untouched {
            tracing::debug!(
                changed_paths = changed.len(),
                files_analysed = self.files_analysed,
                "live pass touched no analysed file; reusing the previous report"
            );
            return Ok(None);
        }
        self.cumulative_stats.hits = self.cumulative_stats.hits.saturating_add(stats.hits);
        self.cumulative_stats.misses = self.cumulative_stats.misses.saturating_add(stats.misses);
        let config = self.pipeline_config(embedding);
        Ok(Some(self.render(&config, stats)?))
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

    /// Updates whether future change passes consult the fingerprint cache.
    /// The config escape hatch still gates the effective value
    /// ([CONFIG-INCREMENTAL-OPTOUT]).
    pub fn set_incremental(&mut self, enabled: bool) {
        self.incremental = enabled;
    }

    /// The requested store mode gated by the live config's escape hatch
    /// ([CONFIG-INCREMENTAL-OPTOUT]) — re-evaluated per pass, so a
    /// `.deslop.toml` edit opting out takes effect on the very next
    /// change pass without a restart. This is the value passes actually
    /// run with, and therefore the value every status surface must
    /// report — surfacing the raw request instead leaves the config
    /// surface claiming a store the passes never consult.
    #[must_use]
    pub fn effective_incremental(&self) -> bool {
        self.incremental && self.exclusion.incremental_enabled()
    }

    /// Returns the total fingerprint count across every live file.
    /// Callers use this to size embedding-progress notifications
    /// before re-running the pass.
    #[must_use]
    pub fn fingerprint_count(&self) -> usize {
        self.store.fingerprint_count()
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
    pub fn exclusion(&self) -> &ExclusionConfig {
        &self.exclusion
    }

    /// Returns a shareable handle to the currently-loaded exclusion
    /// config, so the live watcher applies the same policy the cold scan
    /// resolved ([CONFIG-EXCLUDE-DEPENDENCIES]).
    #[must_use]
    pub fn exclusion_handle(&self) -> Arc<ExclusionConfig> {
        Arc::clone(&self.exclusion)
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

    /// Reloads the exclusion config from disk. Called from
    /// [`Self::update_files`] when a watched config path is in the
    /// changed set ([LIVE-CONFIG-LIVE]).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ConfigParse`] or [`CoreError::ConfigPattern`]
    /// when the new config is malformed. The session keeps the old
    /// config on failure so a bad edit does not brick the daemon.
    fn reload_exclusion(&mut self) -> Result<(), CoreError> {
        self.exclusion = Arc::new(load_exclusion(&self.root, self.config_path.as_deref())?);
        self.ignore_matcher = IgnoreMatcher::build(&self.root);
        Ok(())
    }
}

/// Moves the freshly-parsed per-file bundles into the canonical flat
/// store, feeding them in ascending `(relative path, id)` order so
/// every insert is an append ([PIPELINE-DETERMINISM]).
fn build_store(
    corpus: &mut FingerprintCorpus,
    files: &[DiscoveredFile],
    root: &Path,
) -> CorpusStore {
    let mut keys: Vec<(PathBuf, FileId)> = files
        .iter()
        .map(|file| (store::relative_path_key(&file.path, root), file.file_id))
        .collect();
    keys.sort_unstable();
    let mut built = CorpusStore::default();
    for (path_key, file_id) in keys {
        if let Some(cached) = corpus.per_file.remove(&file_id) {
            built.upsert(file_id, path_key, cached);
        }
    }
    built
}

/// True when a change to `path` re-scopes the corpus rather than
/// contributing content to it: a watched `.deslop.toml` (or explicit
/// override), or an ignore-rule file whose edit re-shapes what the
/// live ingest gate admits ([LIVE-CONFIG-LIVE] #189, ignore-rule
/// parity #287).
fn reshapes_corpus(path: &Path, watched: &[PathBuf]) -> bool {
    is_config_path(path, watched) || is_ignore_rule_path(path)
}

/// Resolves the exclusion config using the session's override path or
/// falling back to the workspace default.
fn load_exclusion(root: &Path, override_path: Option<&Path>) -> Result<ExclusionConfig, CoreError> {
    if let Some(explicit) = override_path {
        return ExclusionConfig::load_for_root(explicit, root);
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
