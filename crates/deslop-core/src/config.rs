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
    path::{Component, Path, PathBuf},
};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Deserialize;

use crate::{error::CoreError, report_metrics::validate_threshold_percent};

/// Default configuration file name searched for next to the scan root.
pub const DEFAULT_CONFIG_FILENAME: &str = ".deslop.toml";

/// Directory components that are always excluded from discovery.
///
/// `.cargo` covers Cargo's vendored registry / git checkout caches
/// ([#142]) — even when the surrounding repo points discovery at
/// the user's home directory by accident, the boilerplate generated
/// code under `.cargo/git/checkouts/...` and
/// `.cargo/registry/src/...` never enters the report.
const BUILTIN_EXCLUDE_COMPONENTS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "__pycache__",
    ".cargo",
];

/// Directory components that are always analysed but hidden from summaries.
const BUILTIN_REPORT_HIDE_COMPONENTS: &[&str] = &["generated"];

/// Non-actionable path component pairs hidden from summaries.
const BUILTIN_REPORT_HIDE_COMPONENT_PAIRS: &[(&str, &str)] = &[
    ("alembic", "versions"),
    ("test", "fixtures"),
    ("tests", "fixtures"),
];

/// File suffixes that are always analysed but hidden from summaries.
const BUILTIN_REPORT_HIDE_SUFFIXES: &[&str] = &[
    ".g.cs",
    ".generated.cs",
    ".designer.cs",
    ".pb.cs",
    ".openapi.cs",
    ".generated.py",
    "_generated.py",
    // Dart code generators (issue #95). `.g.dart` covers source_gen
    // (json_serializable, retrofit, drift, hive, …); the rest cover
    // freezed, auto_route, injectable, flutter_gen, mockito, and the
    // protoc Dart plugin. All carry "GENERATED CODE - DO NOT MODIFY".
    ".g.dart",
    ".freezed.dart",
    ".gr.dart",
    ".config.dart",
    ".gen.dart",
    ".mocks.dart",
    ".pb.dart",
    ".pbenum.dart",
    ".pbjson.dart",
    ".pbserver.dart",
    ".pbgrpc.dart",
];

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
    /// Analysis-wide behavior toggles.
    #[serde(default)]
    analysis: RawAnalysis,
    /// Report-rendering toggles.
    #[serde(default)]
    report: RawReport,
}

/// Raw on-disk shape of the `[analysis]` section.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawAnalysis {
    /// Whether candidate pairs may span different parser language ids.
    #[serde(default)]
    allow_cross_language_comparison: bool,
}

/// Raw on-disk shape of the `[report]` section.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawReport {
    /// Whether the human HTML report divides clusters into per-language
    /// sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). Off by default;
    /// the CLI `--split-by-language` flag also enables it.
    #[serde(default)]
    split_by_language: bool,
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
    /// Whether the HTML report splits clusters into per-language
    /// sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). Defaults off.
    split_by_language: bool,
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
            split_by_language: false,
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
        Ok(Self {
            source: path.to_path_buf(),
            scan_root: scan_root.map(Path::to_path_buf),
            default_exclude,
            default_report_hide,
            per_language,
            default_boilerplate_imports,
            fail_over_percent,
            allow_cross_language_comparison: raw.analysis.allow_cross_language_comparison,
            split_by_language: raw.report.split_by_language,
        })
    }

    /// Returns a copy of this config bound to `scan_root`.
    #[must_use]
    fn with_scan_root(mut self, scan_root: &Path) -> Self {
        self.scan_root = Some(scan_root.to_path_buf());
        self
    }

    /// Returns the `[threshold] max_duplication_percent` loaded from
    /// the config file, if any. `None` means the file did not opt in
    /// to CI gating per [EXIT-CODES].
    #[must_use]
    pub const fn fail_over_percent(&self) -> Option<f64> {
        self.fail_over_percent
    }

    /// Returns whether candidate pairs may span different parser
    /// language ids per [CONFIG-CROSS-LANGUAGE].
    #[must_use]
    pub const fn allows_cross_language_comparison(&self) -> bool {
        self.allow_cross_language_comparison
    }

    /// Returns whether the HTML report should divide clusters into
    /// per-language sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]).
    #[must_use]
    pub const fn split_by_language(&self) -> bool {
        self.split_by_language
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
        if built_in_excluded(path) {
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

/// Returns true when a path is in a built-in ignored dependency or build tree.
fn built_in_excluded(path: &Path) -> bool {
    path_components(path).any(|component| {
        BUILTIN_EXCLUDE_COMPONENTS
            .iter()
            .any(|ignored| component == *ignored)
    })
}

/// Returns true when built-in generated-code rules hide a path from summaries.
fn built_in_report_hidden(path: &Path, scan_root: Option<&Path>) -> bool {
    has_hidden_component(path)
        || has_hidden_component_pair(path, scan_root)
        || has_hidden_suffix(path)
}

/// Returns true when the path has a generated-code directory component.
fn has_hidden_component(path: &Path) -> bool {
    path_components(path).any(|component| {
        BUILTIN_REPORT_HIDE_COMPONENTS
            .iter()
            .any(|hidden| component == *hidden)
    })
}

/// Returns true when the path contains a generated-code component pair.
fn has_hidden_component_pair(path: &Path, scan_root: Option<&Path>) -> bool {
    let components: Vec<String> = path_components(path).collect();
    BUILTIN_REPORT_HIDE_COMPONENT_PAIRS.iter().any(|pair| {
        contains_component_pair(&components, *pair)
            && !scan_root_contains_component_pair(scan_root, *pair)
    })
}

/// Returns true when the scan root itself is inside a hidden component
/// pair. In that case the user intentionally asked to analyse that
/// corpus, so the built-in dogfood hide rule must not erase every
/// positive fixture cluster.
fn scan_root_contains_component_pair(scan_root: Option<&Path>, pair: (&str, &str)) -> bool {
    let Some(root) = scan_root else {
        return false;
    };
    let components: Vec<String> = path_components(root).collect();
    contains_component_pair(&components, pair)
}

/// Returns true when adjacent path components match `pair`.
fn contains_component_pair(components: &[String], pair: (&str, &str)) -> bool {
    components
        .windows(2)
        .any(|window| matches!(window, [first, second] if first == pair.0 && second == pair.1))
}

/// Recognises the unambiguous machine-generated banners build tools emit
/// in a file's head — `@generated` (linguist convention), `GENERATED CODE`
/// (`build_runner` / `source_gen`), `AUTO[- ]GENERATED` (ffigen),
/// `Autogenerated` (jnigen), and `automatically generated` (Flutter/Dart
/// localizations, intl messages, #165). Catches generators that emit no stable file
/// suffix, e.g. ffigen/jnigen FFI bindings, so they join the suffix list in
/// being hidden from the ranked report ([EXCLUSION-CONFIG], #95). Scans only
/// the first kilobyte so a stray phrase deep in hand-written source cannot
/// trip it, and matches ASCII-case-insensitively without allocating.
#[must_use]
pub(crate) fn has_generated_header(source: &[u8]) -> bool {
    const MARKERS: &[&[u8]] = &[
        b"@generated",
        b"generated code",
        b"auto generated",
        b"auto-generated",
        b"autogenerated",
        // Flutter/Dart codegen (generated localizations, intl messages) and
        // a wide class of generators emit "automatically generated … do not
        // edit". Without this marker these dominated worst-offenders on a
        // stock Flutter analysis (#165).
        b"automatically generated",
    ];
    let head = source.get(..source.len().min(1024)).unwrap_or(source);
    MARKERS.iter().any(|marker| {
        head.windows(marker.len())
            .any(|window| window.eq_ignore_ascii_case(marker))
    })
}

/// Returns true when the path's file name has a generated-code suffix.
fn has_hidden_suffix(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) else {
        return false;
    };
    let lower = file_name.to_ascii_lowercase();
    BUILTIN_REPORT_HIDE_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// Returns lowercase normal path components for case-insensitive matching.
fn path_components(path: &Path) -> impl Iterator<Item = String> + '_ {
    path.components().filter_map(|component| match component {
        Component::Normal(value) => value.to_str().map(str::to_ascii_lowercase),
        _ => None,
    })
}

/// Returns `true` when `matcher` classifies `path` as ignored (i.e., the
/// pattern matched). Directories are matched as files here because we
/// only ever feed per-file paths to [`ExclusionConfig::is_excluded`].
fn matches(matcher: &Gitignore, path: &Path) -> bool {
    matcher.matched(path, false).is_ignore()
}

/// Compiles a list of glob patterns into an [`ignore::gitignore::Gitignore`]
/// matcher. The builder is rooted at the scan root when known so user
/// patterns are scan-root-relative — `subdir/**` matches
/// `<scan_root>/subdir/...` regardless of where the scan root sits on
/// disk (#138). With no scan root the matcher falls back to `/` so
/// absolute-path callers still get the original behaviour.
fn build_matcher(
    source: &Path,
    scan_root: Option<&Path>,
    patterns: &[String],
) -> Result<Gitignore, CoreError> {
    let root = scan_root.unwrap_or_else(|| Path::new("/"));
    let mut builder = GitignoreBuilder::new(root);
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
