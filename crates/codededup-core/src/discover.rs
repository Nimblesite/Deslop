//! File discovery.
//!
//! Implements [PIPELINE-DISCOVER-FILES]: walks the target path with the
//! `ignore` crate (honouring `.gitignore` and Git defaults), filters by the
//! set of file extensions contributed by registered language parsers, and
//! registers each discovered path with the run's `FileRegistry`.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::state::{FileId, FileRegistry};

/// A discovered file attached to its [`FileId`].
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path on disk.
    pub path: PathBuf,
    /// Registry handle issued for this file.
    pub file_id: FileId,
    /// Lowercase file extension, without leading `.`.
    pub extension: String,
}

/// Walks `root` and registers every file whose lowercase extension is in
/// `accepted_extensions`. Returns the freshly populated `FileRegistry` and
/// the ordered list of discoveries so downstream stages can stream through
/// them.
#[must_use]
pub fn discover_files(root: &Path, accepted_extensions: &[&str]) -> DiscoveryResult {
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
        if !accepted_extensions
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&extension))
        {
            continue;
        }
        let file_id = registry.register(path.clone());
        files.push(DiscoveredFile {
            path,
            file_id,
            extension,
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
