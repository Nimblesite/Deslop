//! File discovery.
//!
//! Implements [PIPELINE-DISCOVER-FILES]: walks the target path with the
//! `ignore` crate (honouring `.gitignore` and Git defaults), filters by the
//! set of file extensions contributed by registered language parsers, drops
//! files matching [`crate::config::ExclusionConfig`] `exclude` patterns
//! ([EXCLUSION-CONFIG]), and registers each surviving path with the run's
//! `FileRegistry`.

use std::{
    collections::HashMap,
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;

use crate::{
    config::ExclusionConfig,
    state::{FileId, FileRegistry},
};

/// A discovered file attached to its [`FileId`].
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path on disk.
    pub path: PathBuf,
    /// Registry handle issued for this file.
    pub file_id: FileId,
    /// Lowercase file extension, without leading `.`.
    pub extension: String,
    /// Parser language id that claimed this file (e.g. `csharp`,
    /// `rust`, `python`). Used by [`ExclusionConfig`] for per-language
    /// overlays and by the pipeline to route the file to the right
    /// parser.
    pub language: &'static str,
}

/// Walks `root` and registers every file whose lowercase extension is in
/// `extension_to_language`. Files whose absolute path matches a config
/// `exclude` pattern are skipped before registration and are not counted
/// in `files_analysed`.
#[must_use]
pub fn discover_files<S: BuildHasher>(
    root: &Path,
    extension_to_language: &HashMap<String, &'static str, S>,
    config: &ExclusionConfig,
) -> DiscoveryResult {
    let mut registry = FileRegistry::new();
    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .follow_links(false)
        .build();
    for entry_result in walker {
        let Ok(entry) = entry_result else { continue };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        let Some(extension) = lowercase_extension(&path) else {
            continue;
        };
        let Some(language) = extension_to_language.get(&extension).copied() else {
            continue;
        };
        if config.is_excluded(&path, Some(language)) {
            tracing::debug!(
                path = %path.display(),
                language = language,
                "file excluded by config",
            );
            continue;
        }
        let file_id = registry.register(path.clone());
        files.push(DiscoveredFile {
            path,
            file_id,
            extension,
            language,
        });
    }
    DiscoveryResult { registry, files }
}

/// Output of [`discover_files`].
#[derive(Debug)]
pub struct DiscoveryResult {
    /// Populated file registry; ownership passes to the pipeline.
    pub registry: FileRegistry,
    /// Files discovered, in walk order.
    pub files: Vec<DiscoveredFile>,
}

/// Returns the lowercase extension of `path` without the leading `.`, or
/// `None` for files that have no extension or a non-UTF-8 extension.
fn lowercase_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_lowercase)
}
