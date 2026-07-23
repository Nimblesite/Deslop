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

use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    Match, WalkBuilder,
};

use crate::{
    config::ExclusionConfig,
    state::{FileId, FileRegistry},
};

/// Names of the per-directory ignore files the walker honours.
const IGNORE_FILE_NAMES: [&str; 2] = [".gitignore", ".ignore"];

/// Standalone equivalent of the ignore rules [`discover_files`] gets for
/// free from `WalkBuilder::standard_filters(true)` ([PIPELINE-DISCOVER-FILES]).
///
/// The walker prunes ignored *directories* as it descends, so it can only
/// answer "is this path ignored?" while walking. The live ingest path never
/// walks — it is handed individual paths by the file watcher, by LSP
/// `didSave`, and by the freshness tracker — so it needs the same rules as a
/// predicate it can apply to one path at a time.
///
/// Without this, discovery's file set was a strict *subset* of what the live
/// path admitted, and every ignored file touched by a build was ingested into
/// the corpus permanently (#287).
#[derive(Debug)]
pub struct IgnoreMatcher {
    /// Workspace root; hidden-component checks are relative to it so a
    /// dot-directory *above* the workspace never hides the whole tree.
    root: PathBuf,
    /// One compiled matcher per ignore file, paired with the directory that
    /// file governs. Ordered shallowest-first so a deeper rule — including a
    /// `!` whitelist — overrides a shallower one, matching Git's precedence.
    matchers: Vec<(PathBuf, Gitignore)>,
}

impl IgnoreMatcher {
    /// Compiles every `.gitignore` / `.ignore` under `root`, plus
    /// `.git/info/exclude`.
    #[must_use]
    pub fn build(root: &Path) -> Self {
        let mut matchers = collect_ignore_matchers(root);
        let exclude = root.join(".git").join("info").join("exclude");
        if exclude.is_file() {
            push_matcher(&mut matchers, root, &exclude);
        }
        matchers.sort_by_key(|(directory, _)| directory.components().count());
        Self {
            root: root.to_path_buf(),
            matchers,
        }
    }

    /// Returns `true` when the walker would have pruned `path` — because an
    /// ignore rule matches it or any of its parent directories, or because a
    /// path component below the root is hidden (dot-prefixed).
    ///
    /// `path` must be absolute and canonicalised against the same root.
    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> bool {
        if self.has_hidden_component(path) {
            return true;
        }
        self.matchers
            .iter()
            .filter(|(directory, _)| path.starts_with(directory))
            .fold(false, |ignored, (_, matcher)| {
                match matcher.matched_path_or_any_parents(path, false) {
                    Match::Ignore(_) => true,
                    Match::Whitelist(_) => false,
                    Match::None => ignored,
                }
            })
    }

    /// Mirrors the walker's `hidden` filter: any dot-prefixed component
    /// between the workspace root and the file hides it.
    fn has_hidden_component(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.root) else {
            return false;
        };
        relative.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| name.starts_with('.') && name != "." && name != "..")
        })
    }
}

/// Walks `root` and compiles every `.gitignore` / `.ignore` into a matcher
/// paired with the directory it governs.
///
/// Ignore files are themselves hidden, so the walk runs with `hidden(false)`.
/// It keeps the remaining standard filters on, so an ignore file *inside* an
/// already-ignored directory is skipped — exactly as Git treats it.
fn collect_ignore_matchers(root: &Path) -> Vec<(PathBuf, Gitignore)> {
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .follow_links(false)
        .build();
    let mut matchers = Vec::new();
    for entry in walker.filter_map(Result::ok).filter(is_ignore_file) {
        if let Some(parent) = entry.path().parent() {
            push_matcher(&mut matchers, parent, entry.path());
        }
    }
    matchers
}

/// Returns `true` when the walk entry is a `.gitignore` / `.ignore` file.
fn is_ignore_file(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|file_type| file_type.is_file())
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| IGNORE_FILE_NAMES.contains(&name))
}

/// Compiles `file` as an ignore file governing `directory` and appends it.
/// A malformed ignore file is logged and skipped — one bad pattern must never
/// take the daemon down, and skipping only ever admits more files than Git
/// would, never fewer.
fn push_matcher(matchers: &mut Vec<(PathBuf, Gitignore)>, directory: &Path, file: &Path) {
    let mut builder = GitignoreBuilder::new(directory);
    if let Some(error) = builder.add(file) {
        tracing::warn!(%error, "ignore_file_unreadable; rules from it are not applied");
        return;
    }
    match builder.build() {
        Ok(matcher) => matchers.push((directory.to_path_buf(), matcher)),
        Err(error) => {
            tracing::warn!(%error, "ignore_file_malformed; rules from it are not applied");
        }
    }
}

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
