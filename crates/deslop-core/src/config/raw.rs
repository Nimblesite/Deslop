//! Raw on-disk TOML shapes for `.deslop.toml` ([EXCLUSION-CONFIG]).
//! Kept separate from [`super::ExclusionConfig`] so the runtime type
//! can carry compiled matchers instead of raw pattern strings; the
//! compile step lives in [`super`].

use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use super::{BoilerplateImportsMode, ClonePolicy, KindThresholds};
use crate::{
    buckets::{ClusterKind, TotalInclude},
    error::CoreError,
    report_metrics::validate_threshold_percent,
};

/// Raw on-disk TOML shape.
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawConfig {
    /// Shared patterns applied to every language.
    #[serde(default)]
    pub(super) defaults: RawSection,
    /// Per-language pattern overlays, keyed by the parser's language id
    /// (e.g. `csharp`, `rust`, `python`). Patterns extend `defaults`.
    #[serde(default)]
    pub(super) language: HashMap<String, RawSection>,
    /// Opt-in CI gate per [EXIT-CODES]. Populated when a user adds a
    /// `[threshold]` block to `.deslop.toml`.
    #[serde(default)]
    pub(super) threshold: Option<RawThreshold>,
    /// [METRICS-TOTAL-INCLUDE] Which clone kinds count toward the repo
    /// total. Populated when a user adds a `[total]` block.
    #[serde(default)]
    pub(super) total: Option<RawTotal>,
    /// Analysis-wide behavior toggles.
    #[serde(default)]
    pub(super) analysis: RawAnalysis,
    /// Report-rendering toggles.
    #[serde(default)]
    pub(super) report: RawReport,
    /// Clone-category ranking policy ([RANK-CATEGORY]).
    #[serde(default)]
    pub(super) ranking: RawRanking,
    /// [CLONE-BUCKETS-THRESHOLDS] Validated category tuning.
    #[serde(default)]
    pub(super) tuning: super::tuning::Tuning,
}

/// Raw on-disk shape of the `[ranking]` section ([RANK-CATEGORY],
/// [RANK-STRUCTURAL-ONLY]).
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawRanking {
    /// How `data`-category clusters are ranked. `None` means the key was
    /// not set, so the [`ClonePolicy`] default (`demote`) applies.
    #[serde(default)]
    pub(super) data_clones: Option<ClonePolicy>,
    /// Data demote multiplier. `None` means inherit
    /// [`super::DEFAULT_DATA_CLONE_WEIGHT`].
    #[serde(default)]
    pub(super) data_clone_weight: Option<f64>,
    /// How structural-only clusters are ranked. `None` means the key
    /// was not set, so the [`ClonePolicy`] default (`demote`) applies.
    #[serde(default)]
    pub(super) structural_only: Option<ClonePolicy>,
    /// Structural-only demote multiplier. `None` means inherit
    /// [`super::DEFAULT_STRUCTURAL_ONLY_WEIGHT`].
    #[serde(default)]
    pub(super) structural_only_weight: Option<f64>,
}

/// Raw on-disk shape of the `[analysis]` section.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawAnalysis {
    /// Whether candidate pairs may span different parser language ids.
    #[serde(default)]
    pub(super) allow_cross_language_comparison: bool,
    /// Whether third-party library source vendored or installed into the
    /// corpus is analysed ([CONFIG-EXCLUDE-DEPENDENCIES]). Off by default:
    /// ranking is worst-offenders-first, so dependency duplication the
    /// user cannot act on would outrank every first-party finding.
    #[serde(default)]
    pub(super) include_dependencies: bool,
    /// Whether analysis may consult and fill the on-disk parse store
    /// ([CONFIG-INCREMENTAL-OPTOUT]). `true` by default; `false` is the
    /// config-file escape hatch that disables persisted processing for
    /// every surface — CLI, LSP, MCP — without a per-invocation flag.
    #[serde(default = "default_incremental")]
    pub(super) incremental: bool,
    /// Ceiling on a file's mean bytes per line before it is read as build
    /// output rather than source ([CONFIG-EXCLUDE-MINIFIED]). Raise it for a
    /// corpus whose source genuinely runs long lines; lower it to catch
    /// artifacts a bundler wrote less densely.
    #[serde(default = "default_max_average_line_bytes")]
    pub(super) max_average_line_bytes: usize,
}

impl Default for RawAnalysis {
    fn default() -> Self {
        Self {
            allow_cross_language_comparison: false,
            include_dependencies: false,
            incremental: default_incremental(),
            max_average_line_bytes: default_max_average_line_bytes(),
        }
    }
}

/// Persisted processing is on unless the config opts out
/// ([CONFIG-INCREMENTAL-OPTOUT]).
const fn default_incremental() -> bool {
    true
}

/// The shipped mean-bytes-per-line ceiling ([CONFIG-EXCLUDE-MINIFIED]).
const fn default_max_average_line_bytes() -> usize {
    crate::config::DEFAULT_MAX_AVERAGE_LINE_BYTES
}

/// Whether analysis may consult and fill the on-disk parse store
/// ([CONFIG-INCREMENTAL-OPTOUT]). A two-variant enum rather than a
/// bool so the compiled config states the policy by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PersistedProcessing {
    /// Default: the parse store is read and written.
    Enabled,
    /// `[analysis] incremental = false`: the store is never consulted
    /// and never created, whatever the invocation requested.
    Disabled,
}

impl PersistedProcessing {
    /// Maps the raw config key onto the policy.
    pub(super) fn from_key(incremental: bool) -> Self {
        if incremental {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// Raw on-disk shape of the `[report]` section.
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawReport {
    /// Whether the human HTML report divides clusters into per-language
    /// sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). Off by default;
    /// the CLI `--split-by-language` flag also enables it.
    #[serde(default)]
    pub(super) split_by_language: bool,
}

/// Raw on-disk shape of the `[threshold]` section.
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawThreshold {
    /// Percentage above which the analysis run exits `3` per
    /// [EXIT-CODES]. `None` means "key not set" — the gate is off.
    #[serde(default)]
    max_duplication_percent: Option<f64>,
    /// [EXIT-CODES] Per-kind optional limits (`[threshold.kinds]`).
    /// Every key defaults to unset — no per-kind gate exists unless the
    /// user writes one.
    #[serde(default)]
    pub(super) kinds: RawThresholdKinds,
}

/// Raw `[threshold.kinds]` subsection: one optional percentage per
/// clone kind ([EXIT-CODES]). Unknown keys are rejected — a typo'd kind
/// must not silently disable the gate the user meant to set. Shape-only
/// is rejected here too, keeping it a permanent non-clone
/// ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawThresholdKinds {
    /// Limit on the `identical` kind's own percentage.
    #[serde(default)]
    pub(super) identical: Option<f64>,
    /// Limit on the `nearly_identical` kind's own percentage.
    #[serde(default)]
    pub(super) nearly_identical: Option<f64>,
    /// Limit on the `loosely_similar` kind's own percentage.
    #[serde(default)]
    pub(super) loosely_similar: Option<f64>,
    /// Limit on the `same_behavior` kind's own percentage.
    #[serde(default)]
    pub(super) same_behavior: Option<f64>,
}

/// Raw on-disk shape of the `[total]` section ([METRICS-TOTAL-INCLUDE]).
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawTotal {
    /// Wire spellings of the kinds whose lines count toward the repo
    /// total. Absent means the default set
    /// ([`crate::buckets::DEFAULT_TOTAL_INCLUDES`]).
    #[serde(default)]
    pub(super) include: Option<Vec<String>>,
}

/// One TOML section — shared shape across `[defaults]` and
/// `[language.<name>]`.
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawSection {
    /// Patterns whose matches are dropped in [`crate::discover`].
    #[serde(default)]
    pub(super) exclude: Vec<String>,
    /// Patterns whose matches are analysed normally but hidden from the
    /// rendered report.
    #[serde(default)]
    pub(super) report_hide: Vec<String>,
    /// Import/prologue boilerplate policy for this section.
    #[serde(default)]
    pub(super) boilerplate: RawBoilerplate,
}

/// Raw `[*.boilerplate]` subsection.
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct RawBoilerplate {
    /// Import/prologue handling. `None` means inherit/default suppress.
    #[serde(default)]
    pub(super) imports: Option<BoilerplateImportsMode>,
}

/// Validates and returns the `[threshold] max_duplication_percent`
/// value from the raw config, or `None` when the section is absent.
pub(super) fn resolve_threshold(
    source: &Path,
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

/// Validates every `[threshold.kinds]` limit against
/// [`validate_threshold_percent`]; unset keys stay unset. Shape-only has
/// no key to set ([CLONE-BUCKETS-STRUCTURAL-ONLY]) and an unknown key is
/// rejected by serde before this runs.
pub(super) fn resolve_kind_thresholds(
    source: &Path,
    raw: Option<&RawThreshold>,
) -> Result<KindThresholds, CoreError> {
    let Some(block) = raw else {
        return Ok(KindThresholds::default());
    };
    let reject = |message: String| CoreError::ConfigThreshold {
        path: source.to_path_buf(),
        message,
    };
    let validate = |kind: &str, percent: Option<f64>| -> Result<Option<f64>, CoreError> {
        match percent {
            None => Ok(None),
            Some(value) => validate_threshold_percent(value)
                .map(Some)
                .map_err(|msg| reject(format!("[threshold.kinds] {kind}: {msg}"))),
        }
    };
    Ok(KindThresholds {
        identical: validate("identical", block.kinds.identical)?,
        nearly_identical: validate("nearly_identical", block.kinds.nearly_identical)?,
        loosely_similar: validate("loosely_similar", block.kinds.loosely_similar)?,
        same_behavior: validate("same_behavior", block.kinds.same_behavior)?,
    })
}

/// Resolves the `[total] include` set ([METRICS-TOTAL-INCLUDE]):
/// default when the section is absent, otherwise every label must name a
/// clone kind. `structural_only` is refused with a message naming the
/// spec that keeps it a permanent non-clone.
pub(super) fn resolve_total_include(
    source: &Path,
    raw: Option<&RawTotal>,
) -> Result<TotalInclude, CoreError> {
    let Some(block) = raw else {
        return Ok(TotalInclude::default());
    };
    let Some(include) = block.include.as_ref() else {
        return Ok(TotalInclude::default());
    };
    let mut kinds = Vec::with_capacity(include.len());
    for label in include {
        let kind =
            ClusterKind::from_wire_label(label).ok_or_else(|| CoreError::ConfigThreshold {
                path: source.to_path_buf(),
                message: format!(
                    "[total] include: unknown kind '{label}' — expected one of \
                     identical, nearly_identical, loosely_similar, same_behavior"
                ),
            })?;
        if kind == ClusterKind::StructuralOnly {
            return Err(CoreError::ConfigThreshold {
                path: source.to_path_buf(),
                message: "[total] include: 'structural_only' is never duplication \
                     ([CLONE-BUCKETS-STRUCTURAL-ONLY]); it cannot count toward the total"
                    .to_owned(),
            });
        }
        kinds.push(kind);
    }
    Ok(TotalInclude::from_kinds(kinds))
}
