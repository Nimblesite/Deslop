//! Mutation methods for [`super::PipelineSession`].
//!
//! Handles apply-one-change, drop-path, boilerplate replacement, language
//! resolution, path canonicalisation, and pipeline config construction.
//! All methods are `pub(super)` — they are called only from `session/mod.rs`.

use std::path::{Path, PathBuf};

use crate::{
    boilerplate::collect_import_boilerplate_ranges, error::CoreError, report::CacheStats,
    state::FileId,
};

use super::{
    super::{
        config::{EmbeddingSettings, PipelineConfig},
        corpus::{parse_one_file, parser_for_language},
    },
    PipelineSession,
};

impl PipelineSession {
    /// Applies one changed path: delete, update, or add.
    pub(super) fn apply_one_change(
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
        let (cached, source, lines) = match parse_one_file(file_id, &absolute, parser, &config, stats)
        {
            Ok(parsed) => parsed,
            // A pathologically deep file must not crash the long-lived
            // server (#168): drop it and keep serving, the same way an
            // excluded path is handled above. Real parser errors propagate.
            Err(CoreError::AstTooDeep { language, limit }) => {
                tracing::warn!(language, limit, "skipping file: AST nests too deep");
                self.drop_path(&absolute);
                return Ok(());
            }
            Err(other) => return Err(other),
        };
        let ranges = collect_import_boilerplate_ranges(&cached.tree, language);
        self.replace_boilerplate_ranges(file_id, ranges);
        let _prev_lines = self.analysed_lines.insert(file_id, lines);
        let _prev = self.per_file.insert(file_id, cached);
        let _prev_source = self.sources.insert(file_id, source);
        let _prev_path = self.live_paths.insert(file_id, absolute);
        let _prev_lang = self.file_languages.insert(file_id, language);
        self.files_analysed = self.live_paths.len();
        Ok(())
    }

    /// Removes a path from every in-memory map if present.
    pub(super) fn drop_path(&mut self, absolute: &Path) {
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
        self.boilerplate_ranges
            .retain(|range| range.file_id != file_id);
        self.files_analysed = self.live_paths.len();
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
