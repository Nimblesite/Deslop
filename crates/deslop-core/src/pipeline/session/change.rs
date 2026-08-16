//! Mutation methods for [`super::PipelineSession`].
//!
//! Handles apply-one-change, drop-path, boilerplate replacement, language
//! resolution, path canonicalisation, and pipeline config construction.
//! All methods are `pub(super)` — they are called only from `session/mod.rs`.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    boilerplate::collect_import_boilerplate_ranges, discover::discover_files, error::CoreError,
    report::CacheStats, state::FileId,
};

use super::{
    super::{
        config::{EmbeddingSettings, PipelineConfig},
        corpus::{log_skip_too_deep, parse_one_file, parser_for_language, SignatureStats},
    },
    PipelineSession,
};

/// Whether a change pass altered the analysed corpus
/// ([LIVE-SCHEDULER-NOOP]).
///
/// A pass that touches no analysed file cannot change the report, so the
/// whole LSH → embedding → pair → cluster → rank → render chain downstream
/// of it is pure waste and is skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CorpusEffect {
    /// At least one analysed file was added, replaced, or dropped.
    Mutated,
    /// No analysed file was touched; the previous report still holds.
    Untouched,
}

impl CorpusEffect {
    /// Folds two effects together — any mutation wins.
    pub(super) const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Untouched, Self::Untouched) => Self::Untouched,
            _ => Self::Mutated,
        }
    }

    /// Lifts "did this touch anything?" into an effect.
    pub(super) const fn from_touched(touched: bool) -> Self {
        if touched {
            Self::Mutated
        } else {
            Self::Untouched
        }
    }
}

impl PipelineSession {
    /// Applies one changed path: delete, update, or add. Reports whether
    /// the corpus actually moved, so a pass made entirely of paths the
    /// gates below reject can skip the re-render.
    pub(super) fn apply_one_change(
        &mut self,
        path: &Path,
        stats: &mut CacheStats,
        embedding: &EmbeddingSettings<'_>,
    ) -> Result<CorpusEffect, CoreError> {
        let absolute = self.canonicalise_reference(path);
        if !absolute.exists() {
            return Ok(self.drop_subtree(&absolute));
        }
        let Some(language) = self.language_for(&absolute) else {
            return Ok(CorpusEffect::Untouched);
        };
        // Both gates evict, not just skip: a path the corpus already holds
        // must leave it the moment it becomes inadmissible, so an inflated
        // corpus self-heals instead of needing a restart.
        if self.exclusion.is_excluded(&absolute, Some(language)) {
            return Ok(self.drop_path(&absolute));
        }
        // Discovery's walker prunes these; the live path is handed paths
        // directly and must apply the same rules or it admits files
        // discovery never would.
        if self.ignore_matcher.is_ignored(&absolute) {
            return Ok(self.drop_path(&absolute));
        }
        let Some(parser) = parser_for_language(&self.parsers, language) else {
            return Ok(CorpusEffect::Untouched);
        };
        let file_id = self
            .file_id_for(&absolute)
            .unwrap_or_else(|| self.registry.register(absolute.clone()));
        let config = self.pipeline_config_with_mode(embedding);
        let mut signature_stats = SignatureStats::default();
        let (cached, source, lines) = match parse_one_file(
            file_id,
            &absolute,
            parser,
            &config,
            stats,
            &mut signature_stats,
        ) {
            Ok(parsed) => parsed,
            // A pathologically deep file must not crash the long-lived
            // server: drop it and keep serving, the same way an
            // excluded path is handled above. Real parser errors propagate.
            Err(CoreError::AstTooDeep { language, limit }) => {
                log_skip_too_deep(language, limit);
                return Ok(self.drop_path(&absolute));
            }
            Err(other) => return Err(other),
        };
        if self
            .sources
            .get(&file_id)
            .is_some_and(|previous| previous == &source)
        {
            tracing::info!(
                source_bytes = source.len(),
                fingerprints = cached.fingerprints.len(),
                "live change skipped; source bytes unchanged"
            );
            return Ok(CorpusEffect::Untouched);
        }
        tracing::debug!(
            fingerprints = cached.fingerprints.len(),
            signatures_built = signature_stats.built,
            signatures_reused = signature_stats.reused,
            "live change spliced into corpus",
        );
        let ranges = collect_import_boilerplate_ranges(&cached.tree, language);
        self.replace_boilerplate_ranges(file_id, ranges);
        let _prev_lines = self.analysed_lines.insert(file_id, lines);
        let _prev = self.per_file.insert(file_id, cached);
        let _prev_source = self.sources.insert(file_id, source);
        let _prev_path = self.live_paths.insert(file_id, absolute);
        let _prev_lang = self.file_languages.insert(file_id, language);
        self.files_analysed = self.live_paths.len();
        Ok(CorpusEffect::Mutated)
    }

    /// Reloads `.deslop.toml` and re-evaluates the existing corpus
    /// against it: drops files the new `exclude` patterns now match
    /// and re-discovers files a removed pattern re-admits
    /// ([LIVE-CONFIG-LIVE]). A malformed config is logged and
    /// the prior exclusion is kept — a typo never bricks the daemon.
    pub(super) fn refresh_exclusion(
        &mut self,
        stats: &mut CacheStats,
        embedding: &EmbeddingSettings<'_>,
    ) -> Result<(), CoreError> {
        if let Err(error) = self.reload_exclusion() {
            tracing::warn!(
                %error,
                "deslop_toml_reload_failed; keeping prior exclusion config",
            );
            return Ok(());
        }
        let dropped = self.drop_newly_excluded();
        let added = self.apply_newly_included(stats, embedding)?;
        tracing::info!(dropped, added, "deslop_toml_reloaded corpus re-evaluated");
        Ok(())
    }

    /// Drops every live file the reloaded exclusion config — or the
    /// rebuilt ignore matcher — now matches, so a `.deslop.toml` or
    /// `.gitignore` edit evicts exactly what a fresh discovery would
    /// prune (ignore-rule parity #287). Returns the number of dropped
    /// paths.
    fn drop_newly_excluded(&mut self) -> usize {
        let doomed: Vec<PathBuf> = self
            .live_paths
            .iter()
            .filter_map(|(file_id, path)| {
                let language = self.file_languages.get(file_id).copied();
                let excluded = self.exclusion.is_excluded(path, language)
                    || self.ignore_matcher.is_ignored(path);
                excluded.then(|| path.clone())
            })
            .collect();
        for path in &doomed {
            let _effect = self.drop_path(path);
        }
        doomed.len()
    }

    /// Re-runs discovery and applies every path the reloaded exclusion
    /// config re-admits but the corpus does not yet contain. Returns
    /// the number of newly-applied paths.
    fn apply_newly_included(
        &mut self,
        stats: &mut CacheStats,
        embedding: &EmbeddingSettings<'_>,
    ) -> Result<usize, CoreError> {
        let discovery = discover_files(&self.root, &self.extension_to_language, &self.exclusion);
        let known: HashSet<PathBuf> = self.live_paths.values().cloned().collect();
        let fresh: Vec<PathBuf> = discovery
            .files
            .into_iter()
            .map(|file| file.path)
            .filter(|path| !known.contains(path))
            .collect();
        for path in &fresh {
            let _effect = self.apply_one_change(path, stats, embedding)?;
        }
        Ok(fresh.len())
    }

    /// Evicts every live file at or under `prefix`. A directory removal
    /// reports only the directory path (no source extension, matching no
    /// live leaf), so the session must drop each registered descendant
    /// to keep the report in sync ([LIVE-WATCHER]). Component-wise
    /// [`Path::starts_with`] means a removed `pkg` never evicts a
    /// sibling `pkg_twin`. The prefix is reflexive, so an exact-leaf
    /// deletion drops just that file. Reuses [`Self::drop_path`] per
    /// match so all maps, boilerplate ranges, and `files_analysed` stay
    /// consistent. Reports whether anything was actually evicted — a
    /// removal event for a path the corpus never held leaves it
    /// [`CorpusEffect::Untouched`].
    pub(super) fn drop_subtree(&mut self, prefix: &Path) -> CorpusEffect {
        let doomed: Vec<PathBuf> = self
            .live_paths
            .values()
            .filter(|registered| registered.starts_with(prefix))
            .cloned()
            .collect();
        doomed.iter().fold(CorpusEffect::Untouched, |effect, path| {
            effect.merge(self.drop_path(path))
        })
    }

    /// Removes a path from every in-memory map, reporting whether it was
    /// present to begin with.
    pub(super) fn drop_path(&mut self, absolute: &Path) -> CorpusEffect {
        let Some((file_id, _)) = self
            .live_paths
            .iter()
            .find(|(_, registered)| registered.as_path() == absolute)
            .map(|(id, path)| (*id, path.clone()))
        else {
            return CorpusEffect::Untouched;
        };
        let _removed_path = self.live_paths.remove(&file_id);
        let _removed_cache = self.per_file.remove(&file_id);
        let _removed_source = self.sources.remove(&file_id);
        let _removed_lang = self.file_languages.remove(&file_id);
        let _removed_lines = self.analysed_lines.remove(&file_id);
        self.boilerplate_ranges
            .retain(|range| range.file_id != file_id);
        self.files_analysed = self.live_paths.len();
        CorpusEffect::Mutated
    }

    /// Replaces all remembered boilerplate ranges for one live file.
    pub(super) fn replace_boilerplate_ranges(
        &mut self,
        file_id: FileId,
        ranges: Vec<crate::boilerplate::BoilerplateRange>,
    ) {
        self.boilerplate_ranges
            .retain(|range| range.file_id != file_id);
        self.boilerplate_ranges.extend(ranges);
    }

    /// Returns the registered language id that claims `path`, if any.
    pub(super) fn language_for(&self, path: &Path) -> Option<&'static str> {
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_lowercase)?;
        self.extension_to_language.get(&extension).copied()
    }

    /// Resolves `path` against the workspace root so relative paths
    /// from a watcher are handled identically to absolute paths.
    /// Canonicalises so symlinks resolve the same way as the canonical
    /// [`self.root`] — without this the macOS `/var/...` →
    /// `/private/var/...` symlink pair leaves watcher paths missing
    /// the registry entries they should match ([#141 MCP-SAFETY]).
    /// For paths whose leaf no longer exists (deletions) we
    /// canonicalise the parent and rejoin the leaf so removals still
    /// hit the registry.
    pub(super) fn canonicalise_reference(&self, path: &Path) -> PathBuf {
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        if let Ok(resolved) = std::fs::canonicalize(&joined) {
            return resolved;
        }
        match (joined.parent(), joined.file_name()) {
            (Some(parent), Some(leaf)) => match std::fs::canonicalize(parent) {
                Ok(canonical_parent) => canonical_parent.join(leaf),
                Err(_) => joined,
            },
            _ => joined,
        }
    }

    /// Builds a [`PipelineConfig`] snapshot for a pass that does not
    /// need the embedding provider — the parse-only cache-consulting
    /// path in [`parse_one_file`].
    pub(super) fn pipeline_config_with_mode<'a>(
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
                batch_yield: embedding.batch_yield,
                progress: embedding.progress,
            },
            incremental: self.incremental,
        }
    }

    /// Builds a [`PipelineConfig`] that owns the provider reference
    /// and is suitable for [`crate::pipeline::embedding_pass::run_embedding_pass`].
    pub(super) fn pipeline_config<'a>(
        &self,
        embedding: EmbeddingSettings<'a>,
    ) -> PipelineConfig<'a> {
        PipelineConfig {
            root: self.root.clone(),
            min_nodes: self.min_nodes,
            config_path: self.config_path.clone(),
            embedding,
            incremental: self.incremental,
        }
    }
}
