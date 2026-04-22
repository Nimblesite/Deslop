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

use crate::{error::CoreError, report_metrics::validate_threshold_percent};

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

/// Raw on-disk TOML shape. Kept separate from [`ExclusionConfig`] so the
/// runtime type can carry compiled matchers instead of raw pattern strings.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawConfig {
    /// Shared patterns applied to every language.
    #[serde(default)]
    defaults: RawSection,
    /// Per-language pattern overlays, keyed by the parser's language id
    /// (e.g. `csharp`, `rust`, `python`). Patterns extend `defaults`.
    #[serde(default)]
    language: HashMap<String, RawSection>,
    /// Opt-in CI gate per [EXIT-CODES]. Populated when a user adds a
    /// `[threshold]` block to `.deslop.toml`.
    #[serde(default)]
    threshold: Option<RawThreshold>,
}

/// Raw on-disk shape of the `[threshold]` section.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawThreshold {
    /// Percentage above which the analysis run exits `3` per
    /// [EXIT-CODES]. `None` means "key not set" — the gate is off.
    #[serde(default)]
    max_duplication_percent: Option<f64>,
}

/// One TOML section — shared shape across `[defaults]` and
/// `[language.<name>]`.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawSection {
    /// Patterns whose matches are dropped in [`crate::discover`].
    #[serde(default)]
    exclude: Vec<String>,
    /// Patterns whose matches are analysed normally but hidden from the
    /// rendered report.
    #[serde(default)]
    report_hide: Vec<String>,
    /// Import/prologue boilerplate policy for this section.
    #[serde(default)]
    boilerplate: RawBoilerplate,
}

/// Raw `[*.boilerplate]` subsection.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawBoilerplate {
    /// Import/prologue handling. `None` means inherit/default suppress.
    #[serde(default)]
    imports: Option<BoilerplateImportsMode>,
}

/// Compiled exclusion configuration ready for matching. Built by merging
/// per-language sections onto the defaults so a `.cs` file is tested
/// against `defaults.exclude ∪ language.csharp.exclude` without the
/// caller having to check both.
#[derive(Debug)]
pub struct ExclusionConfig {
    /// Source path of the config that produced this value, for diagnostics.
    source: PathBuf,
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
            default_exclude: empty_matcher(),
            default_report_hide: empty_matcher(),
            per_language: HashMap::new(),
            default_boilerplate_imports: BoilerplateImportsMode::Suppress,
            fail_over_percent: None,
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
        let source = fs::read_to_string(path).map_err(|err| CoreError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let raw: RawConfig = toml::from_str(&source).map_err(|err| CoreError::ConfigParse {
            path: path.to_path_buf(),
            source: err,
        })?;
        Self::compile(path, &raw)
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
            Self::load(&candidate)
        } else {
            Ok(Self::empty())
        }
    }

    /// Compiles a validated [`RawConfig`] into matcher form.
    fn compile(path: &Path, raw: &RawConfig) -> Result<Self, CoreError> {
        let default_exclude = build_matcher(path, &raw.defaults.exclude)?;
        let default_report_hide = build_matcher(path, &raw.defaults.report_hide)?;
        let default_boilerplate_imports = raw
            .defaults
            .boilerplate
            .imports
            .unwrap_or(BoilerplateImportsMode::Suppress);
        let mut per_language: HashMap<String, LanguageMatchers> = HashMap::new();
        for (language, section) in &raw.language {
            let exclude = build_matcher(path, &section.exclude)?;
            let report_hide = build_matcher(path, &section.report_hide)?;
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
        Ok(Self {
            source: path.to_path_buf(),
            default_exclude,
            default_report_hide,
            per_language,
            default_boilerplate_imports,
            fail_over_percent,
        })
    }

    /// Returns the `[threshold] max_duplication_percent` loaded from
    /// the config file, if any. `None` means the file did not opt in
    /// to CI gating per [EXIT-CODES].
    #[must_use]
    pub const fn fail_over_percent(&self) -> Option<f64> {
        self.fail_over_percent
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

/// Compiles a list of glob patterns into an [`ignore::gitignore::Gitignore`]
/// matcher. The builder is rooted at `/` so absolute paths (which is what
/// [`crate::discover`] produces) match the same way they would in a
/// repo-wide `.gitignore`.
fn build_matcher(source: &Path, patterns: &[String]) -> Result<Gitignore, CoreError> {
    let mut builder = GitignoreBuilder::new("/");
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

/// Validates and returns the `[threshold] max_duplication_percent`
/// value from the raw config, or `None` when the section is absent.
fn resolve_threshold(
    source: &std::path::Path,
    raw: Option<&RawThreshold>,
) -> Result<Option<f64>, CoreError> {
    let Some(percent) = raw.and_then(|block| block.max_duplication_percent) else {
        return Ok(None);
    };
    validate_threshold_percent(percent)
        .map(Some)
        .map_err(|msg| CoreError::ConfigThreshold {
            path: source.to_path_buf(),
            message: msg,
        })
}
