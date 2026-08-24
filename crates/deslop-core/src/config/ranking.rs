//! The `[ranking]` clone-category policy surface ([RANK-CATEGORY],
//! [RANK-STRUCTURAL-ONLY]): the three-way [`ClonePolicy`], the compiled
//! [`RankingPolicy`] with its validated demote multipliers, and the
//! raw-config resolution that builds it.

use std::path::Path;

use serde::Deserialize;

use crate::error::CoreError;

use super::raw::RawRanking;

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
    pub(super) fn with_global_override(mut self) -> Self {
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

/// Validates and compiles the `[ranking]` section into a [`RankingPolicy`]
/// ([RANK-CATEGORY], [RANK-STRUCTURAL-ONLY]). Both knobs default to
/// `demote` with their class default weight; an explicit weight must be
/// finite and strictly inside `(0.0, 1.0]` or the load fails with a
/// `ConfigThreshold`-style error.
pub(super) fn resolve_ranking_policy(
    source: &Path,
    raw: &RawRanking,
) -> Result<RankingPolicy, CoreError> {
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
