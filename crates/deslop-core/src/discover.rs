//! File discovery.
//!
//! Implements [PIPELINE-DISCOVER-FILES]: walks the target path with the
//! `ignore` crate (honouring `.gitignore` and Git defaults), filters by the
//! set of file extensions contributed by registered language parsers, drops
//! files matching [`crate::config::ExclusionConfig`] `exclude` patterns
//! ([EXCLUSION-CONFIG]), and registers each surviving path with the run's
//! `FileRegistry`.

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
    path::{Path, PathBuf},
    sync::Arc,
};

use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    Match, WalkBuilder,
};

use crate::{
    config::ExclusionConfig,
    error::CoreError,
    state::{indexed_subtree_paths, FileId, FileRegistry},
};

#[cfg(test)]
mod tests;

/// Names of the per-directory ignore files the walker honours.
const IGNORE_FILE_NAMES: [&str; 2] = [".gitignore", ".ignore"];

/// Compiled rules grouped by their governing directory, retaining same-directory precedence.
type DirectoryMatchers = BTreeMap<PathBuf, Vec<Gitignore>>;

/// Standalone equivalent of the ignore rules [`discover_files`] gets for
/// free from `WalkBuilder::standard_filters(true)` ([PIPELINE-DISCOVER-FILES]).
///
/// The walker prunes ignored *directories* as it descends, so it can only
/// answer "is this path ignored?" while walking. Live ingest also receives
/// individual paths from the watcher, LSP `didSave`, and the freshness tracker,
/// so it needs the same rules as a predicate for one path at a time.
///
/// Without this, discovery's file set was a strict *subset* of what the live
/// path admitted, and every ignored file touched by a build was ingested into
/// the corpus permanently.
#[derive(Debug)]
pub struct IgnoreMatcher {
    /// Workspace root; hidden-component checks are relative to it so a
    /// dot-directory *above* the workspace never hides the whole tree.
    root: PathBuf,
    /// Rules indexed by governing directory for bounded refresh, removal, and ancestor lookup.
    matchers: DirectoryMatchers,
}

impl IgnoreMatcher {
    /// Compiles every `.gitignore` / `.ignore` under `root`, plus
    /// `.git/info/exclude`.
    #[must_use]
    pub fn build(root: &Path) -> Self {
        Self::from_matchers(root, collect_ignore_matchers(root))
    }

    /// [LIVE-WATCHER-RESCAN] Builds complete rules without traversing excluded directories.
    pub(crate) fn build_configured(
        root: &Path,
        config: &Arc<ExclusionConfig>,
    ) -> Result<Self, CoreError> {
        let walker = scoped_walker(root, root, Some(Arc::clone(config)))
            .hidden(false)
            .build();
        let matchers = compile_ignore_entries(complete_walk(walker, root)?.into_iter());
        Ok(Self::from_matchers(root, matchers))
    }

    /// Preserves the repository-local exclude file and shallow-to-deep rule ordering.
    fn from_matchers(root: &Path, mut matchers: DirectoryMatchers) -> Self {
        let exclude = root.join(".git").join("info").join("exclude");
        if exclude.is_file() {
            push_matcher(&mut matchers, root, &exclude);
        }
        Self {
            root: root.to_path_buf(),
            matchers,
        }
    }

    /// [LIVE-WATCHER-DIRECTORIES] Refreshes rules below an admitted changed directory.
    pub(crate) fn refresh_subtree(
        &mut self,
        subtree: &Path,
        config: &Arc<ExclusionConfig>,
    ) -> Result<(), CoreError> {
        let walker = scoped_walker(&self.root, subtree, Some(Arc::clone(config)))
            .hidden(false)
            .build();
        let matchers = compile_ignore_entries(complete_walk(walker, subtree)?.into_iter());
        self.remove_subtree(subtree);
        self.matchers.extend(matchers);
        Ok(())
    }

    /// [LIVE-WATCHER-DIRECTORIES] Removes only rules governed by a deleted directory.
    pub(crate) fn remove_subtree(&mut self, subtree: &Path) {
        let (directories, _candidates_visited) = indexed_subtree_paths(&self.matchers, subtree);
        for directory in directories {
            let _removed = self.matchers.remove(&directory);
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
        let ancestors: Vec<_> = path.ancestors().collect();
        ancestors
            .into_iter()
            .rev()
            .filter_map(|directory| self.matchers.get(directory))
            .flatten()
            .fold(false, |ignored, matcher| {
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
fn collect_ignore_matchers(root: &Path) -> DirectoryMatchers {
    collect_ignore_matchers_in(root, root)
}

/// Retains workspace ignore context when compiling a newly populated subtree.
fn collect_ignore_matchers_in(root: &Path, subtree: &Path) -> DirectoryMatchers {
    let walker = scoped_walker(root, subtree, None).hidden(false).build();
    compile_ignore_entries(walker.filter_map(Result::ok))
}

/// Compiles the canonical ignore-file stream once traversal policy has admitted it.
fn compile_ignore_entries(entries: impl Iterator<Item = ignore::DirEntry>) -> DirectoryMatchers {
    let mut matchers = DirectoryMatchers::new();
    for entry in entries.filter(is_ignore_file) {
        if let Some(parent) = entry.path().parent() {
            push_matcher(&mut matchers, parent, entry.path());
        }
    }
    matchers
}

/// Returns `true` when the walk entry is a `.gitignore` / `.ignore` file.
fn is_ignore_file(entry: &ignore::DirEntry) -> bool {
    entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
        && has_ignore_file_name(entry.path())
}

/// Returns `true` when `path` is named `.gitignore` / `.ignore`.
fn has_ignore_file_name(path: &Path) -> bool {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|name| IGNORE_FILE_NAMES.contains(&name))
}

/// Returns `true` when `path` names an ignore-rule file this matcher
/// compiles: `.gitignore` / `.ignore` anywhere in the tree, or the
/// repository's `.git/info/exclude`. An edit to any of them re-scopes
/// what the live gate admits, so the watcher and the session treat them
/// like watched config paths ([LIVE-WATCHER]).
#[must_use]
pub fn is_ignore_rule_path(path: &Path) -> bool {
    has_ignore_file_name(path) || path.ends_with(Path::new(".git/info/exclude"))
}

/// Compiles `file` as an ignore file governing `directory` and appends it.
/// A malformed ignore file is logged and skipped — one bad pattern must never
/// take the daemon down, and skipping only ever admits more files than Git
/// would, never fewer.
fn push_matcher(matchers: &mut DirectoryMatchers, directory: &Path, file: &Path) {
    let mut builder = GitignoreBuilder::new(directory);
    if let Some(error) = builder.add(file) {
        tracing::warn!(%error, "ignore_file_unreadable; rules from it are not applied");
        return;
    }
    match builder.build() {
        Ok(matcher) => matchers
            .entry(directory.to_path_buf())
            .or_default()
            .push(matcher),
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
    let walker = scoped_walker(root, root, None).build();
    collect_discovered(walker.filter_map(Result::ok), extension_to_language, config)
}

/// [LIVE-WATCHER-DIRECTORIES] Scopes traversal while retaining workspace ignore context.
pub(crate) fn discover_files_in<S: BuildHasher>(
    root: &Path,
    subtree: &Path,
    extension_to_language: &HashMap<String, &'static str, S>,
    config: &Arc<ExclusionConfig>,
) -> Result<DiscoveryResult, CoreError> {
    let walker = scoped_walker(root, subtree, Some(Arc::clone(config))).build();
    collect_complete(walker, subtree, extension_to_language, config)
}

/// Collects a scoped inventory before the live session applies its membership diff.
fn collect_complete<S: BuildHasher>(
    walker: impl IntoIterator<Item = Result<ignore::DirEntry, ignore::Error>>,
    subtree: &Path,
    extensions: &HashMap<String, &'static str, S>,
    config: &ExclusionConfig,
) -> Result<DiscoveryResult, CoreError> {
    let entries = complete_walk(walker, subtree)?;
    Ok(collect_discovered(entries.into_iter(), extensions, config))
}

/// [LIVE-WATCHER-RESCAN] Incomplete discovery must never authorize membership eviction.
fn complete_walk(
    walker: impl IntoIterator<Item = Result<ignore::DirEntry, ignore::Error>>,
    subtree: &Path,
) -> Result<Vec<ignore::DirEntry>, CoreError> {
    walker
        .into_iter()
        .collect::<Result<_, _>>()
        .map_err(|error| CoreError::Io {
            path: subtree.to_path_buf(),
            source: std::io::Error::other(error),
        })
}

/// Prunes unrelated branches without rebasing anchored or ancestor ignore rules.
fn scoped_walker(root: &Path, subtree: &Path, config: Option<Arc<ExclusionConfig>>) -> WalkBuilder {
    let subtree = subtree.to_path_buf();
    let mut walker = WalkBuilder::new(root);
    let _configured = walker
        .standard_filters(true)
        .follow_links(false)
        .filter_entry(move |entry| {
            subtree.starts_with(entry.path())
                || (entry.path().starts_with(&subtree)
                    && admits_directory(entry, config.as_deref()))
        });
    walker
}

/// Skips excluded directories before their descendants or ignore files are visited.
fn admits_directory(entry: &ignore::DirEntry, config: Option<&ExclusionConfig>) -> bool {
    !entry.file_type().is_some_and(|kind| kind.is_dir())
        || config.map_or(true, |config| !config.is_excluded(entry.path(), None))
}

/// Registers the canonical discovery stream after language and exclusion admission.
fn collect_discovered<S: BuildHasher>(
    walker: impl Iterator<Item = ignore::DirEntry>,
    extensions: &HashMap<String, &'static str, S>,
    config: &ExclusionConfig,
) -> DiscoveryResult {
    let mut registry = FileRegistry::new();
    let files = walker
        .filter_map(|entry| eligible_file(entry, extensions, config))
        .map(|metadata| register_discovered(metadata, &mut registry))
        .collect();
    DiscoveryResult { registry, files }
}

/// Applies the same file, extension, language, and config gates to every walk.
fn eligible_file<S: BuildHasher>(
    entry: ignore::DirEntry,
    extensions: &HashMap<String, &'static str, S>,
    config: &ExclusionConfig,
) -> Option<(PathBuf, String, &'static str)> {
    if !entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
    {
        return None;
    }
    let path = entry.into_path();
    let extension = lowercase_extension(&path)?;
    let language = extensions.get(&extension).copied()?;
    (!config.is_excluded(&path, Some(language))).then_some((path, extension, language))
}

/// Attaches the registry identity after a discovered file has passed admission.
fn register_discovered(
    (path, extension, language): (PathBuf, String, &'static str),
    registry: &mut FileRegistry,
) -> DiscoveredFile {
    DiscoveredFile {
        file_id: registry.register(path.clone()),
        path,
        extension,
        language,
    }
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
