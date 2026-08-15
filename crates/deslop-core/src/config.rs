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

use crate::{
    error::CoreError,
    report_metrics::{validate_threshold_percent, ThresholdSource, ThresholdSummary},
};

/// Default configuration file name searched for next to the scan root.
pub const DEFAULT_CONFIG_FILENAME: &str = ".deslop.toml";

/// Directory components that are always excluded from discovery.
///
/// `.cargo` covers Cargo's vendored registry / git checkout caches
/// — even when the surrounding repo points discovery at
/// the user's home directory by accident, the boilerplate generated
/// code under `.cargo/git/checkouts/...` and
/// `.cargo/registry/src/...` never enters the report.
///
/// `.git` and `.claude` cover working-tree copies that look like source
/// but are not actionable duplication. Claude Code agent
/// workflows create full git worktrees under `.claude/worktrees/<id>/`;
/// each is another checkout of the same repo, so without this exclusion
/// every file is reported as N identical "copies". The initial walk's
/// hidden-dir filter skips dot-dirs, but the live watcher
/// (`live/watcher.rs`) and incremental update (`pipeline/session/change.rs`)
/// have no hidden filter and rely solely on this component list, so the
/// exclusion must live here to cover all three discovery paths.
///
/// `.dart_tool` and `.pub-cache` cover the Dart/Flutter toolchain's own
/// caches: `.dart_tool/` holds `package_config.json`, `build_runner`
/// outputs, and per-package generated `.dart`; `.pub-cache/` is a
/// vendored copy of every dependency's source. On a large Flutter
/// monorepo a hot build churns thousands of `.dart` files under
/// `.dart_tool/`; because the live watcher has no `.gitignore` filter,
/// excluding them here is what keeps that churn from monopolising the
/// session and starving the editor's responsiveness.
///
/// `vendor` is the third-party source copy for Go (`go mod vendor`),
/// PHP (Composer) and Rust (`cargo vendor`). Unlike every other entry it
/// is *conventionally committed* — GitHub's `Go.gitignore` ships the
/// `vendor/` rule commented out — and it is not dot-prefixed, so neither
/// the gitignore pass nor the hidden-directory pass prunes it. Without
/// this entry a vendored repo hands the pipeline tens of thousands of
/// dependency files, and because ranking is worst-offenders-first the
/// resulting third-party duplication outranks every first-party finding
/// the user can actually act on.
/// Third-party *library source* vendored or installed into the corpus.
/// These are real, readable source files the user did not write, so
/// analysing them is a legitimate — if unusual — request: auditing a
/// dependency for duplication, or checking whether first-party code
/// duplicates a library it already depends on. Governed by
/// `[analysis] include_dependencies` ([CONFIG-EXCLUDE-DEPENDENCIES]),
/// which defaults to `false`.
const BUILTIN_DEPENDENCY_COMPONENTS: &[&str] =
    &["node_modules", "vendor", ".cargo", ".pub-cache", ".venv"];

/// Build output, tool caches, and working-tree copies
/// ([CONFIG-EXCLUDE-BUILTIN]). Excluded unconditionally: none of it is
/// source the user wrote, and none of it is a "library the code depends
/// on", so no configuration opts back in.
/// `target`, `dist`, `build`, `__pycache__` and `.dart_tool` are compiler
/// and codegen output; `.git` and `.claude` are whole additional checkouts
/// of the same repository which would otherwise report every file
/// as N identical copies.
const BUILTIN_ARTEFACT_COMPONENTS: &[&str] = &[
    "target",
    "dist",
    "build",
    "__pycache__",
    ".dart_tool",
    ".git",
    ".claude",
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
    // Dart code generators. `.g.dart` covers source_gen
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

/// How a demotable clone class is ranked. One shared three-way policy
/// serves both `[ranking]` knobs: `data_clones` for `data`-category
/// clusters ([RANK-CATEGORY]) and `structural_only` for shape-only
/// evidence clusters ([RANK-STRUCTURAL-ONLY]). Both default to
/// [`Self::Demote`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClonePolicy {
    /// Down-weight matching clusters by the class's weight multiplier
    /// so they rank below comparable full-evidence clones but stay in
    /// the report, labelled. The default.
    #[default]
    Demote,
    /// Drop matching clusters from the report entirely (counted in
    /// `clusters_hidden`).
    Ignore,
    /// Rank matching clusters at full weight.
    Keep,
}

impl std::str::FromStr for ClonePolicy {
    type Err = String;

    /// Parses the CLI/editor-settings spelling of a policy
    /// ([VSIX-SETTINGS-RANKING]): `demote`, `ignore`, or `keep`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "demote" => Ok(Self::Demote),
            "ignore" => Ok(Self::Ignore),
            "keep" => Ok(Self::Keep),
            other => Err(format!("expected demote|ignore|keep, got {other:?}")),
        }
    }
}

/// Default `data_clone_weight` multiplier in [`ClonePolicy::Demote`]
/// ([RANK-CATEGORY]). Kept above zero so a pathologically large verbatim
/// data blob can still rise rather than being silently zeroed.
pub const DEFAULT_DATA_CLONE_WEIGHT: f64 = 0.15;

/// Default `structural_only_weight` multiplier in
/// [`ClonePolicy::Demote`] ([RANK-STRUCTURAL-ONLY]). Matches the data
/// default: shape-only families sink below comparable token- or
/// semantics-supported clones, but a pathologically large family can
/// still rise rather than being silently zeroed.
pub const DEFAULT_STRUCTURAL_ONLY_WEIGHT: f64 = 0.15;

/// Compiled `[ranking]` policy ([RANK-CATEGORY],
/// [RANK-STRUCTURAL-ONLY]). Carries the validated demote multipliers so
/// callers never re-validate at render time.
#[derive(Debug, Clone, Copy)]
pub struct RankingPolicy {
    /// Selected three-way data-clone policy.
    data_clones: ClonePolicy,
    /// Validated data multiplier; finite and strictly inside `(0.0, 1.0]`.
    data_clone_weight: f64,
    /// Selected three-way structural-only policy.
    structural_only: ClonePolicy,
    /// Validated structural-only multiplier; finite and strictly inside
    /// `(0.0, 1.0]`.
    structural_only_weight: f64,
}

impl Default for RankingPolicy {
    fn default() -> Self {
        Self {
            data_clones: ClonePolicy::Demote,
            data_clone_weight: DEFAULT_DATA_CLONE_WEIGHT,
            structural_only: ClonePolicy::Demote,
            structural_only_weight: DEFAULT_STRUCTURAL_ONLY_WEIGHT,
        }
    }
}

impl RankingPolicy {
    /// Returns the selected data-clone policy.
    #[must_use]
    pub const fn data_clones(self) -> ClonePolicy {
        self.data_clones
    }

    /// Returns the selected structural-only policy.
    #[must_use]
    pub const fn structural_only(self) -> ClonePolicy {
        self.structural_only
    }

    /// Multiplier applied to a `data`-category cluster's ranking weight
    /// ([RANK-CATEGORY]). `1.0` for [`ClonePolicy::Keep`] (no demotion);
    /// the validated `data_clone_weight` for [`ClonePolicy::Demote`].
    /// [`ClonePolicy::Ignore`] never reweighs — those clusters are
    /// dropped — so it reports `1.0` for completeness.
    #[must_use]
    pub fn data_weight_multiplier(self) -> f64 {
        multiplier_for(self.data_clones, self.data_clone_weight)
    }

    /// Multiplier applied to a structural-only cluster's ranking weight
    /// ([RANK-STRUCTURAL-ONLY]); same `demote`/`ignore`/`keep`
    /// semantics as [`Self::data_weight_multiplier`].
    #[must_use]
    pub fn structural_only_weight_multiplier(self) -> f64 {
        multiplier_for(self.structural_only, self.structural_only_weight)
    }

    /// True when `data`-category clusters must be dropped from the report
    /// entirely rather than demoted.
    #[must_use]
    pub fn drops_data_clusters(self) -> bool {
        matches!(self.data_clones, ClonePolicy::Ignore)
    }

    /// True when structural-only clusters must be dropped from the
    /// report entirely rather than demoted ([RANK-STRUCTURAL-ONLY]).
    #[must_use]
    pub fn drops_structural_only(self) -> bool {
        matches!(self.structural_only, ClonePolicy::Ignore)
    }

    /// Applies the process-wide [RANK-STRUCTURAL-ONLY] override from
    /// [`crate::state`], when one was recorded at startup. The
    /// editor-settings channel ([VSIX-SETTINGS-RANKING]) wins over
    /// `.deslop.toml`.
    #[must_use]
    fn with_global_override(mut self) -> Self {
        if let Some(policy) = crate::state::structural_only_override() {
            self.structural_only = policy;
        }
        self
    }
}

/// Shared demote/ignore/keep → multiplier mapping for one policy knob.
fn multiplier_for(policy: ClonePolicy, demote_weight: f64) -> f64 {
    match policy {
        ClonePolicy::Demote => demote_weight,
        ClonePolicy::Keep | ClonePolicy::Ignore => 1.0,
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
    /// Clone-category ranking policy ([RANK-CATEGORY]).
    #[serde(default)]
    ranking: RawRanking,
}

/// Raw on-disk shape of the `[ranking]` section ([RANK-CATEGORY],
/// [RANK-STRUCTURAL-ONLY]).
#[derive(Debug, Default, Clone, Deserialize)]
struct RawRanking {
    /// How `data`-category clusters are ranked. `None` means the key was
    /// not set, so the [`ClonePolicy`] default (`demote`) applies.
    #[serde(default)]
    data_clones: Option<ClonePolicy>,
    /// Data demote multiplier. `None` means inherit
    /// [`DEFAULT_DATA_CLONE_WEIGHT`].
    #[serde(default)]
    data_clone_weight: Option<f64>,
    /// How structural-only clusters are ranked. `None` means the key
    /// was not set, so the [`ClonePolicy`] default (`demote`) applies.
    #[serde(default)]
    structural_only: Option<ClonePolicy>,
    /// Structural-only demote multiplier. `None` means inherit
    /// [`DEFAULT_STRUCTURAL_ONLY_WEIGHT`].
    #[serde(default)]
    structural_only_weight: Option<f64>,
}

/// Raw on-disk shape of the `[analysis]` section.
#[derive(Debug, Default, Clone, Deserialize)]
struct RawAnalysis {
    /// Whether candidate pairs may span different parser language ids.
    #[serde(default)]
    allow_cross_language_comparison: bool,
    /// Whether third-party library source vendored or installed into the
    /// corpus is analysed ([CONFIG-EXCLUDE-DEPENDENCIES]). Off by default:
    /// ranking is worst-offenders-first, so dependency duplication the
    /// user cannot act on would outrank every first-party finding.
    #[serde(default)]
    include_dependencies: bool,
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
    /// Whether third-party library source inside the corpus is analysed
    /// ([CONFIG-EXCLUDE-DEPENDENCIES]). Defaults off.
    include_dependencies: bool,
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

/// Returns true when `path` sits in a built-in excluded tree **inside the
/// analysed corpus** ([CONFIG-EXCLUDE-BUILTIN]).
///
/// Only components at or below `scan_root` are considered. The rule prunes
/// dependency and build trees *within the corpus the user asked for*; a
/// directory name above the scan root describes where the checkout happens
/// to sit on disk and says nothing about its contents. Matching those
/// ancestors excluded every file in any repository nested under e.g.
/// `dist`, `build`, `target`, `vendor` or `node_modules`, yielding
/// `files_analysed: 0`, `clusters: []`, `threshold.breached: false` and a
/// successful exit — a total, silent false negative. This mirrors the
/// carve-out [`scan_root_contains_component_pair`] already applies to the
/// report-hide tier.
///
/// When `include_dependencies` is set, [`BUILTIN_DEPENDENCY_COMPONENTS`]
/// stops applying and third-party library source enters the analysis;
/// [`BUILTIN_ARTEFACT_COMPONENTS`] applies regardless
/// ([CONFIG-EXCLUDE-DEPENDENCIES]).
///
/// A path outside `scan_root`, or a config with no known root, is not part
/// of any corpus this rule governs, so the rule does not fire. That
/// direction can only admit a file for analysis — it can never silently
/// discard one.
///
/// Pinned by `crates/deslop/tests/issue_342_scan_root_under_excluded_ancestor.rs`
/// and `crates/deslop/tests/go_vendor_exclusion.rs`.
fn corpus_built_in_excluded(
    path: &Path,
    scan_root: Option<&Path>,
    include_dependencies: bool,
) -> bool {
    let Some(components) = corpus_components(path, scan_root) else {
        return false;
    };
    components.into_iter().any(|component| {
        BUILTIN_ARTEFACT_COMPONENTS
            .iter()
            .chain(dependency_components(include_dependencies))
            .any(|excluded| component == *excluded)
    })
}

/// The dependency component list when it applies, or nothing when the user
/// opted into analysing libraries ([CONFIG-EXCLUDE-DEPENDENCIES]).
fn dependency_components(include_dependencies: bool) -> &'static [&'static str] {
    if include_dependencies {
        &[]
    } else {
        BUILTIN_DEPENDENCY_COMPONENTS
    }
}

/// Returns the lowercased components of `path` lying strictly below
/// `scan_root` — the part of the path the user asked deslop to analyse.
/// `None` when no corpus boundary is known or `path` sits outside it.
fn corpus_components(path: &Path, scan_root: Option<&Path>) -> Option<Vec<String>> {
    let relative = scan_root.and_then(|root| path.strip_prefix(root).ok())?;
    Some(path_components(relative).collect())
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
/// localizations, intl messages). Catches generators that emit no stable file
/// suffix, e.g. ffigen/jnigen FFI bindings, so they join the suffix list in
/// being hidden from the ranked report ([EXCLUSION-CONFIG]). Scans only
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
        // stock Flutter analysis.
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

/// Validates and compiles the `[ranking]` section into a [`RankingPolicy`]
/// ([RANK-CATEGORY], [RANK-STRUCTURAL-ONLY]). Both knobs default to
/// `demote` with their class default weight; an explicit weight must be
/// finite and strictly inside `(0.0, 1.0]` or the load fails with a
/// `ConfigThreshold`-style error.
fn resolve_ranking_policy(source: &Path, raw: &RawRanking) -> Result<RankingPolicy, CoreError> {
    let data_clone_weight = resolve_clone_weight(
        source,
        raw.data_clone_weight,
        "data_clone_weight",
        DEFAULT_DATA_CLONE_WEIGHT,
    )?;
    let structural_only_weight = resolve_clone_weight(
        source,
        raw.structural_only_weight,
        "structural_only_weight",
        DEFAULT_STRUCTURAL_ONLY_WEIGHT,
    )?;
    Ok(RankingPolicy {
        data_clones: raw.data_clones.unwrap_or_default(),
        data_clone_weight,
        structural_only: raw.structural_only.unwrap_or_default(),
        structural_only_weight,
    })
}

/// Validates one optional `[ranking]` weight, inheriting `default`
/// when the key is absent and failing the load with a
/// `ConfigThreshold`-style error otherwise.
fn resolve_clone_weight(
    source: &Path,
    raw: Option<f64>,
    key: &str,
    default: f64,
) -> Result<f64, CoreError> {
    let Some(weight) = raw else {
        return Ok(default);
    };
    validate_clone_weight(weight, key).map_err(|message| CoreError::ConfigThreshold {
        path: source.to_path_buf(),
        message,
    })
}

/// Returns `weight` when it is a finite multiplier strictly inside
/// `(0.0, 1.0]`, else a diagnostic explaining the rejection. Zero is
/// rejected so a demoted cluster can never be silently erased; values above
/// `1.0` would *promote* the demoted class, defeating the policy
/// ([RANK-CATEGORY], [RANK-STRUCTURAL-ONLY]).
fn validate_clone_weight(weight: f64, key: &str) -> Result<f64, String> {
    if !weight.is_finite() {
        return Err(format!("{key} must be finite, got {weight}"));
    }
    if weight <= 0.0 || weight > 1.0 {
        return Err(format!(
            "{key} must be in the range (0.0, 1.0], got {weight}"
        ));
    }
    Ok(weight)
}
