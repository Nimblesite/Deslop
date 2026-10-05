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
    /// Builds one report from a stable editor-buffer snapshot, before embedding.
    pub(crate) fn initialise_with_buffers(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding: EmbeddingSettings<'_>,
        buffers: &OpenBuffers,
    ) -> Result<(Self, Report), CoreError> {
        Self::initialise_from_sources(
            root,
            min_nodes,
            incremental,
            config_path,
            embedding,
            None,
            buffers,
        )
    }

    /// Shared cold pass; editor sources are installed before any report is rendered.
    pub(super) fn initialise_from_sources(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding: EmbeddingSettings<'_>,
        diff: Option<&crate::diff_scope::ParsedDiff>,
        buffers: &OpenBuffers,
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
        let mut live_paths = LivePaths::default();
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
