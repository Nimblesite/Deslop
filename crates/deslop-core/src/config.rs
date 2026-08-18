//! Exclusion configuration (`[EXCLUSION-CONFIG]`).
//!
//! Two orthogonal tiers:
//!
//! - **`exclude`** — pattern matches drop a file from [`crate::discover`]
//!   entirely. Never parsed, never fingerprinted.
//! - **`report_hide`** — pattern matches analyse the file normally but
//!   mark every occurrence `hidden = true` at render time; a cluster whose
//!   members are *all* hidden is dropped from the rendered output.
//!
//! The split makes "regular code duplicates generated code" visible (the
//! cluster has at least one non-hidden occurrence so it survives) while
//! keeping the "generated code duplicates itself" family of clusters from
//! dominating the report header.
//!
//! Config format is TOML with one shared `[defaults]` block plus optional
//! per-language `[language.<name>]` blocks. Per-language patterns
//! **extend** the defaults, they do not replace them.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Deserialize;

use crate::{
    error::CoreError,
    report_metrics::{ThresholdSource, ThresholdSummary},
};

pub(crate) use builtin::has_generated_header;
use builtin::{built_in_report_hidden, corpus_built_in_excluded};
use ranking::resolve_ranking_policy;
pub use ranking::{
    ClonePolicy, RankingPolicy, DEFAULT_DATA_CLONE_WEIGHT, DEFAULT_STRUCTURAL_ONLY_WEIGHT,
};
use raw::{resolve_threshold, PersistedProcessing, RawConfig};

mod builtin;
mod ranking;
mod raw;

/// Default configuration file name searched for next to the scan root.
pub const DEFAULT_CONFIG_FILENAME: &str = ".deslop.toml";

/// Import/prologue boilerplate reporting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoilerplateImportsMode {
    /// Suppress import/prologue-only clones and emit no hygiene hints.
    Suppress,
    /// Suppress clone warnings but emit structured low-severity hints.
    Report,
}

impl BoilerplateImportsMode {
    /// Returns true when structured report hints should be emitted.
    #[must_use]
    pub const fn reports_hints(self) -> bool {
        matches!(self, Self::Report)
    }
}

/// Compiled exclusion configuration ready for matching. Built by merging
/// per-language sections onto the defaults so a `.cs` file is tested
/// against `defaults.exclude ∪ language.csharp.exclude` without the
/// caller having to check both.
#[derive(Debug)]
pub struct ExclusionConfig {
    /// Source path of the config that produced this value, for diagnostics.
    source: PathBuf,
    /// Scan root the config applies to, when known. Built-in summary
    /// hiding uses this to avoid hiding a fixture corpus when it is the
    /// explicit target being analysed.
    scan_root: Option<PathBuf>,
    /// Shared exclude matcher applied regardless of language.
    default_exclude: Gitignore,
    /// Shared report-hide matcher applied regardless of language.
    default_report_hide: Gitignore,
    /// Per-language overlay matchers, keyed by parser language id.
    per_language: HashMap<String, LanguageMatchers>,
    /// Shared import/prologue boilerplate mode.
    default_boilerplate_imports: BoilerplateImportsMode,
    /// Optional fail-over threshold loaded from `[threshold]
    /// max_duplication_percent` per [EXIT-CODES]. `None` means the
    /// config file did not opt in.
    fail_over_percent: Option<f64>,
    /// Whether candidate pairs may span different parser language ids
    /// ([CONFIG-CROSS-LANGUAGE]). Defaults off to keep reports focused
    /// on same-language refactoring.
    allow_cross_language_comparison: bool,
    /// Whether third-party library source inside the corpus is analysed
    /// ([CONFIG-EXCLUDE-DEPENDENCIES]). Defaults off.
    include_dependencies: bool,
    /// Whether analysis may consult and fill the on-disk parse store
    /// ([CONFIG-INCREMENTAL-OPTOUT]). Defaults to
    /// [`PersistedProcessing::Enabled`].
    incremental: PersistedProcessing,
    /// Whether the HTML report splits clusters into per-language
    /// sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). Defaults off.
    split_by_language: bool,
    /// Compiled clone-category ranking policy ([RANK-CATEGORY]).
    ranking_policy: RankingPolicy,
}

/// Compiled matchers for a single language overlay.
#[derive(Debug)]
struct LanguageMatchers {
    /// Language-specific exclude matcher; additive with `default_exclude`.
    exclude: Gitignore,
    /// Language-specific report-hide matcher; additive with
    /// `default_report_hide`.
    report_hide: Gitignore,
    /// Optional language-specific boilerplate import mode.
    boilerplate_imports: Option<BoilerplateImportsMode>,
}

impl ExclusionConfig {
    /// An empty configuration — nothing excluded, nothing hidden. Used
    /// when no config file is present.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            source: PathBuf::new(),
            scan_root: None,
            default_exclude: empty_matcher(),
            default_report_hide: empty_matcher(),
            per_language: HashMap::new(),
            default_boilerplate_imports: BoilerplateImportsMode::Suppress,
            fail_over_percent: None,
            allow_cross_language_comparison: false,
            include_dependencies: false,
            incremental: PersistedProcessing::Enabled,
            split_by_language: false,
            ranking_policy: RankingPolicy::default().with_global_override(),
        }
    }

    /// Loads the config from an explicit file path.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Io`] when the file cannot be read,
    /// [`CoreError::ConfigParse`] when the TOML is invalid, and
    /// [`CoreError::ConfigPattern`] when a pattern is rejected by
    /// `ignore::gitignore`.
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        Self::load_with_root(path, None)
    }

    /// Loads the config from an explicit file path for a known scan
    /// root.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`ExclusionConfig::load`].
    pub fn load_for_root(path: &Path, scan_root: &Path) -> Result<Self, CoreError> {
        Self::load_with_root(path, Some(scan_root))
    }

    /// Shared config loader used by root-aware and rootless call sites.
    fn load_with_root(path: &Path, scan_root: Option<&Path>) -> Result<Self, CoreError> {
        let source = fs::read_to_string(path).map_err(|err| CoreError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let raw: RawConfig = toml::from_str(&source).map_err(|err| CoreError::ConfigParse {
            path: path.to_path_buf(),
            source: err,
        })?;
        Self::compile(path, scan_root, &raw)
    }

    /// Searches `scan_root` for [`DEFAULT_CONFIG_FILENAME`]. When found,
    /// loads it. When absent, returns the empty config — missing config
    /// is not an error.
    ///
    /// # Errors
    ///
    /// Forwards any failure from [`ExclusionConfig::load`] when a config
    /// file exists but cannot be parsed.
    pub fn discover(scan_root: &Path) -> Result<Self, CoreError> {
        let candidate = scan_root.join(DEFAULT_CONFIG_FILENAME);
        if candidate.is_file() {
            Self::load_for_root(&candidate, scan_root)
        } else {
            Ok(Self::empty().with_scan_root(scan_root))
        }
    }

    /// Compiles a validated [`RawConfig`] into matcher form.
    fn compile(path: &Path, scan_root: Option<&Path>, raw: &RawConfig) -> Result<Self, CoreError> {
        let default_exclude = build_matcher(path, scan_root, &raw.defaults.exclude)?;
        let default_report_hide = build_matcher(path, scan_root, &raw.defaults.report_hide)?;
        let default_boilerplate_imports = raw
            .defaults
            .boilerplate
            .imports
            .unwrap_or(BoilerplateImportsMode::Suppress);
        let mut per_language: HashMap<String, LanguageMatchers> = HashMap::new();
        for (language, section) in &raw.language {
            let exclude = build_matcher(path, scan_root, &section.exclude)?;
            let report_hide = build_matcher(path, scan_root, &section.report_hide)?;
            let _previous = per_language.insert(
                language.clone(),
                LanguageMatchers {
                    exclude,
                    report_hide,
                    boilerplate_imports: section.boilerplate.imports,
                },
            );
        }
        let fail_over_percent = resolve_threshold(path, raw.threshold.as_ref())?;
        let ranking_policy = resolve_ranking_policy(path, &raw.ranking)?.with_global_override();
        Ok(Self {
            source: path.to_path_buf(),
            scan_root: scan_root.map(Path::to_path_buf),
            default_exclude,
            default_report_hide,
            per_language,
            default_boilerplate_imports,
            fail_over_percent,
            allow_cross_language_comparison: raw.analysis.allow_cross_language_comparison,
            include_dependencies: raw.analysis.include_dependencies,
            incremental: PersistedProcessing::from_key(raw.analysis.incremental),
            split_by_language: raw.report.split_by_language,
            ranking_policy,
        })
    }

    /// Returns a copy of this config bound to `scan_root`.
    ///
    /// Binding the root is required for accurate built-in exclusion: it is
    /// what confines [`corpus_built_in_excluded`] to the analysed corpus
    /// instead of the whole filesystem path. Watcher call sites that
    /// build an [`ExclusionConfig::empty`] must bind their root here.
    #[must_use]
    pub fn with_scan_root(mut self, scan_root: &Path) -> Self {
        self.scan_root = Some(scan_root.to_path_buf());
        self
    }

    /// Resolves the config `[threshold] max_duplication_percent` into a
    /// verdict against the `measured` repo-wide duplication percentage.
    /// `None` config yields the "no gate" summary. This is the single
    /// place a config threshold becomes a [`ThresholdSummary`], so the
    /// live LSP/MCP render path and the CLI agree ([EXIT-CODES]).
    #[must_use]
    pub fn resolve_threshold(&self, measured: f64) -> ThresholdSummary {
        match self.fail_over_percent {
            Some(percent) => ThresholdSummary::resolve(percent, ThresholdSource::Config, measured),
            None => ThresholdSummary::none(),
        }
    }

    /// Returns whether candidate pairs may span different parser
    /// language ids per [CONFIG-CROSS-LANGUAGE].
    #[must_use]
    pub const fn allows_cross_language_comparison(&self) -> bool {
        self.allow_cross_language_comparison
    }

    /// Returns whether analysis may consult and fill the on-disk parse
    /// store ([CONFIG-INCREMENTAL-OPTOUT]). `false` is the config-file
    /// escape hatch: it overrides whatever the invocation requested, on
    /// every surface, so persisted processing can always be turned off
    /// without touching a flag. Pinned by
    /// `signature_reuse.rs::config_file_opt_out_disables_persisted_processing`.
    #[must_use]
    pub fn incremental_enabled(&self) -> bool {
        self.incremental == PersistedProcessing::Enabled
    }

    /// Returns whether the HTML report should divide clusters into
    /// per-language sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]).
    #[must_use]
    pub const fn split_by_language(&self) -> bool {
        self.split_by_language
    }

    /// Returns the compiled clone-category ranking policy ([RANK-CATEGORY]).
    #[must_use]
    pub const fn ranking_policy(&self) -> RankingPolicy {
        self.ranking_policy
    }

    /// Returns the source path this config was loaded from, or an empty
    /// path for [`ExclusionConfig::empty`]. Diagnostic only.
    #[must_use]
    pub fn source_path(&self) -> &Path {
        &self.source
    }

    /// True when `path` matches any `exclude` pattern (language-specific
    /// or shared). `path` is the absolute discovered path; matching is
    /// performed against it directly so `ignore::gitignore` semantics
    /// match how [`crate::discover`] walks the tree.
    #[must_use]
    pub fn is_excluded(&self, path: &Path, language: Option<&str>) -> bool {
        if corpus_built_in_excluded(path, self.scan_root.as_deref(), self.include_dependencies) {
            return true;
        }
        if matches(&self.default_exclude, path) {
            return true;
        }
        if let Some(lang) = language {
            if let Some(overlay) = self.per_language.get(lang) {
                if matches(&overlay.exclude, path) {
                    return true;
                }
            }
        }
        false
    }

    /// True when `path` matches any `report_hide` pattern under the
    /// given `language` overlay (or the shared `default` set). Hidden
    /// files are still analysed — the flag only affects rendering.
    #[must_use]
    pub fn is_report_hidden(&self, path: &Path, language: &str) -> bool {
        if built_in_report_hidden(path, self.scan_root.as_deref()) {
            return true;
        }
        if matches(&self.default_report_hide, path) {
            return true;
        }
        self.per_language
            .get(language)
            .is_some_and(|overlay| matches(&overlay.report_hide, path))
    }

    /// Returns the import/prologue boilerplate reporting mode for a language.
    #[must_use]
    pub fn boilerplate_imports_mode(&self, language: &str) -> BoilerplateImportsMode {
        self.per_language
            .get(language)
            .and_then(|overlay| overlay.boilerplate_imports)
            .unwrap_or(self.default_boilerplate_imports)
    }
}

/// Returns `true` when `matcher` classifies `path` as ignored (i.e., the
/// pattern matched). Directories are matched as files here because we
/// only ever feed per-file paths to [`ExclusionConfig::is_excluded`].
fn matches(matcher: &Gitignore, path: &Path) -> bool {
    matcher.matched(path, false).is_ignore()
}

/// Builds the canonical set of config paths that should trigger a live
/// exclusion reload — `<root>/.deslop.toml` plus the explicit override
/// (if any) ([LIVE-CONFIG-LIVE]).
#[must_use]
pub fn watched_config_paths(root: &Path, override_path: Option<&Path>) -> Vec<PathBuf> {
    let default = root.join(DEFAULT_CONFIG_FILENAME);
    let mut paths = vec![canonicalise_or_clone(&default)];
    if let Some(explicit) = override_path {
        paths.push(canonicalise_or_clone(explicit));
    }
    paths
}

/// Returns `true` when `candidate` matches one of the watched config
/// paths in either canonical or as-given form.
#[must_use]
pub fn is_config_path(candidate: &Path, watched: &[PathBuf]) -> bool {
    if watched.iter().any(|watched_path| watched_path == candidate) {
        return true;
    }
    let canonical = canonicalise_or_clone(candidate);
    watched
        .iter()
        .any(|watched_path| watched_path == &canonical)
}

/// Canonicalises `path` when possible; otherwise returns a clone so
/// non-existent override paths still compare predictably.
fn canonicalise_or_clone(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Compiles a list of glob patterns into an [`ignore::gitignore::Gitignore`]
/// matcher. The builder is rooted at the scan root when known so user
/// patterns are scan-root-relative — `subdir/**` matches
/// `<scan_root>/subdir/...` regardless of where the scan root sits on
/// disk. With no scan root the matcher falls back to `/` so
/// absolute-path callers still get the original behaviour.
///
/// Unclosed character classes are rejected here even though
/// `GitignoreBuilder` began allowing them in `ignore` 0.4.31 to match Git,
/// which reads a dangling `[` literally. That leniency is right for the
/// `.gitignore` files Git owns ([`crate::discover`] keeps it) and wrong for
/// `.deslop.toml`, which only Deslop reads: `exclude = ["[unclosed"]` is a
/// typo, and silently compiling it into a literal filename match excludes
/// nothing while looking like it worked. Config errors are reported, never
/// no-opped ([EXCLUSION-CONFIG]).
fn build_matcher(
    source: &Path,
    scan_root: Option<&Path>,
    patterns: &[String],
) -> Result<Gitignore, CoreError> {
    let root = scan_root.unwrap_or_else(|| Path::new("/"));
    let mut builder = GitignoreBuilder::new(root);
    let _ = builder.allow_unclosed_class(false);
    for pattern in patterns {
        if let Err(err) = builder.add_line(None, pattern) {
            return Err(CoreError::ConfigPattern {
                path: source.to_path_buf(),
                pattern: pattern.clone(),
                source: err,
            });
        }
    }
    builder.build().map_err(|err| CoreError::ConfigPattern {
        path: source.to_path_buf(),
        pattern: String::new(),
        source: err,
    })
}

/// Empty matcher used by [`ExclusionConfig::empty`]. Never matches
/// anything.
fn empty_matcher() -> Gitignore {
    Gitignore::empty()
}
