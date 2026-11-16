//! Initial analysis from disk and editor sources ([LIVE-OPEN-BUFFERS]).

use super::{build_store, load_exclusion, log_discovery_summary, PipelineSession};
use crate::{
    discover::{discover_files, IgnoreMatcher},
    error::CoreError,
    pipeline::{
        corpus::{build_extension_map, default_parsers, fingerprint_corpus},
        EmbeddingSettings, OpenBuffers, PipelineConfig,
    },
    report::Report,
    state::{FileId, LivePaths},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

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

    /// [`Self::initialise`] for a session that will take edits through
    /// [`Self::update_files`]: it keeps the pairs each render discovers so
    /// an edit re-pairs only what it touched
    /// ([PIPELINE-INCREMENTAL-ANALYSIS-PAIRS]).
    ///
    /// # Errors
    ///
    /// Everything [`Self::initialise`] produces.
    pub fn initialise_for_edits(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding: EmbeddingSettings<'_>,
    ) -> Result<(Self, Report), CoreError> {
        let config = PipelineConfig {
            root,
            min_nodes,
            config_path,
            embedding,
            incremental,
        };
        Self::initialise_from_sources(config, None, &OpenBuffers::default(), true)
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
        let config = PipelineConfig {
            root,
            min_nodes,
            config_path,
            embedding,
            incremental,
        };
        Self::initialise_from_sources(config, diff, &OpenBuffers::default(), false)
    }

    /// Builds one report from a stable editor-buffer snapshot, before
    /// embedding, for a live session that takes edits.
    #[cfg(feature = "live")]
    pub(crate) fn initialise_with_buffers(
        config: PipelineConfig<'_>,
        buffers: &OpenBuffers,
    ) -> Result<(Self, Report), CoreError> {
        Self::initialise_from_sources(config, None, buffers, true)
    }

    /// Shared cold pass; editor sources are installed before any report is rendered.
    pub(super) fn initialise_from_sources(
        config: PipelineConfig<'_>,
        diff: Option<&crate::diff_scope::ParsedDiff>,
        buffers: &OpenBuffers,
        takes_edits: bool,
    ) -> Result<(Self, Report), CoreError> {
        let PipelineConfig {
            root,
            min_nodes,
            config_path,
            embedding,
            incremental,
        } = config;
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
        let discovery = discover_files(&root, &extension_to_language, &exclusion)?;
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
        let mut live_paths = LivePaths::default();
        let mut file_languages: HashMap<FileId, &'static str> = HashMap::new();
        let analysed = discovery
            .files
            .iter()
            .filter(|discovered| !corpus.skipped.contains_key(&discovered.path));
        for discovered in analysed {
            let _prev = live_paths.insert(discovered.file_id, discovered.path.clone());
            let _prev_language = file_languages.insert(discovered.file_id, discovered.language);
        }
        let files_analysed = live_paths.len();
        let mut session = Self {
            osprey_projects: discovery.projects,
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
            pair_ledger: None,
            takes_edits,
            sources: corpus.sources,
            live_paths,
            file_languages,
            cumulative_stats: corpus.cache_stats,
            analysed_lines: corpus.analysed_lines,
            boilerplate_ranges: corpus.boilerplate_ranges,
            files_analysed,
            skipped_files: std::mem::take(&mut corpus.skipped),
            diff_scope: None,
        };
        // [LIVE-OPEN-BUFFERS] Overlay editor text before rendering or embedding.
        let mut stats = corpus.cache_stats;
        for path in buffers.paths() {
            let _effect = session.apply_one_change(path, &mut stats, &config.embedding, buffers)?;
        }
        if let Some(parsed) = diff {
            session.attach_diff(parsed)?;
        }
        let report = session.render(&config, stats)?;
        Ok((session, report))
    }
}
