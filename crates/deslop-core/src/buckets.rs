//! The clone-kind registry ([CLONE-KIND-LABELS]): one label set per
//! [`ClusterKind`], the strength order the cluster fold reads
//! ([CLONE-KIND-FOLD]), and the pair-content support floors that
//! admission and explicit comparison share ([FUSED-CONTENT-GATE]).

pub use crate::wire_generated::ClusterKind;
use crate::wire_generated::PairClassification;
pub(crate) mod grouping;

/// Default pair-content support floor ([FUSED-CONTENT-GATE]).
pub const CONTENT_SUPPORT_FLOOR: f64 = 0.7;

/// [METRICS-TOTAL-INCLUDE] The kinds whose lines count toward the repo
/// total when `[total] include` is absent: the two byte-level relations
/// only. Looser findings stay in the report with their own per-kind
/// figures but leave the headline number.
pub const DEFAULT_TOTAL_INCLUDES: [ClusterKind; 2] =
    [ClusterKind::Identical, ClusterKind::NearlyIdentical];

/// [METRICS-TOTAL-INCLUDE] Reports predating the include field counted every clone kind.
#[must_use]
pub fn all_clone_kinds() -> Vec<ClusterKind> {
    ClusterKind::all()
        .into_iter()
        .filter(|kind| kind.is_clone())
        .collect()
}

/// Stronger pair-content support floor an unanchored LSH-only pair pays
/// in every scope ([FUSED-CONTENT-GATE]).
pub const CONTENT_PROMOTE_FLOOR: f64 = 0.85;

/// Returns the independent pair-content support `max(A, R)`.
#[must_use]
pub fn content_support(agreement: f64, rename_consistency: f64) -> f64 {
    agreement.max(rename_consistency)
}

/// The human labels of one clone kind ([CLONE-KIND-LABELS]). Every
/// surface that names a kind reads them from here, so the HTML report,
/// the terminal summary, the LSP diagnostic and the extension agree on
/// the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindLabels {
    /// The title a cluster surface shows, e.g. `Identical code`. Never
    /// carries advice.
    pub title: &'static str,
    /// The clone-taxonomy name ([CLONE-TYPE-TAXONOMY]), for tooltips and
    /// agent context.
    pub taxonomy: &'static str,
    /// The CSS class suffix the HTML report keys its colours on.
    pub css_suffix: &'static str,
}

impl ClusterKind {
    /// [SEVERITY-DESLOP-MAP] Shared defaults; mass never selects diagnostic severity.
    #[must_use]
    pub const fn default_diagnostic_severity(self) -> &'static str {
        match self {
            Self::Identical | Self::NearlyIdentical => "warning",
            Self::SameBehavior | Self::LooselySimilar => "information",
            Self::StructuralOnly => "none",
        }
    }

    /// [CLONE-BUCKETS-STRUCTURAL-ONLY] Shape-only is informational, not duplication.
    #[must_use]
    pub const fn is_clone(self) -> bool {
        !matches!(self, Self::StructuralOnly)
    }

    /// Every kind, in the category display order — Identical,
    /// Nearly identical, Similar, Same behavior, then shape-only — the
    /// one order every surface lists kinds in ([CLONE-KIND-LABELS]).
    /// This is presentation order only; [`Self::weaker`] folds by
    /// [`Self::strength`], a separate concern ([CLONE-KIND-FOLD]).
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Identical,
            Self::NearlyIdentical,
            Self::LooselySimilar,
            Self::SameBehavior,
            Self::StructuralOnly,
        ]
    }

    /// Parses one `[total] include` / `[threshold.kinds]` key from its
    /// stable wire spelling ([METRICS-TOTAL-INCLUDE]). Unknown spellings
    /// yield `None` so the config layer can name the offending key.
    #[must_use]
    pub fn from_wire_label(label: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|kind| kind.wire_label() == label)
    }

    /// This kind's position in the category display order
    /// ([CLONE-KIND-LABELS]) — the sort key for include sets on the wire.
    #[must_use]
    pub fn display_position(self) -> usize {
        Self::all()
            .into_iter()
            .position(|kind| kind == self)
            .unwrap_or_else(|| Self::all().len())
    }

    /// [CLONE-KIND-FOLD] An unclassified comparison supplies no clone kind.
    #[must_use]
    pub const fn from_pair(classification: Option<PairClassification>) -> Option<Self> {
        match classification {
            Some(PairClassification::Identical) => Some(Self::Identical),
            Some(PairClassification::NearlyIdentical) => Some(Self::NearlyIdentical),
            Some(PairClassification::SameBehavior) => Some(Self::SameBehavior),
            Some(PairClassification::StructuralOnly) => Some(Self::StructuralOnly),
            Some(PairClassification::LooselySimilar) => Some(Self::LooselySimilar),
            None => None,
        }
    }

    /// The weaker of two kinds ([CLONE-KIND-FOLD]).
    #[must_use]
    pub const fn weaker(self, other: Self) -> Self {
        if other.strength() < self.strength() {
            other
        } else {
            self
        }
    }

    /// How much a kind claims: the byte-proven kind claims the most, a
    /// relation carried only through other members the least.
    const fn strength(self) -> u8 {
        match self {
            Self::StructuralOnly => 0,
            Self::LooselySimilar => 1,
            Self::SameBehavior => 2,
            Self::NearlyIdentical => 3,
            Self::Identical => 4,
        }
    }

    /// The stable wire spelling, identical to the serde form.
    #[must_use]
    pub const fn wire_label(self) -> &'static str {
        match self {
            Self::Identical => "identical",
            Self::NearlyIdentical => "nearly_identical",
            Self::SameBehavior => "same_behavior",
            Self::StructuralOnly => "structural_only",
            Self::LooselySimilar => "loosely_similar",
        }
    }

    /// The human labels of this kind ([CLONE-KIND-LABELS]).
    #[must_use]
    pub const fn labels(self) -> KindLabels {
        match self {
            Self::Identical => KindLabels {
                title: "Identical code",
                taxonomy: "Type-1 exact clone",
                css_suffix: "identical",
            },
            Self::NearlyIdentical => KindLabels {
                title: "Nearly identical code",
                taxonomy: "Type-2 / close Type-3 clone",
                css_suffix: "nearly-identical",
            },
            Self::SameBehavior => KindLabels {
                title: "Same behavior, different code",
                taxonomy: "Type-4 semantic clone",
                css_suffix: "same-behavior",
            },
            Self::StructuralOnly => KindLabels {
                title: "Same shape, different content",
                taxonomy: "Informational non-clone",
                css_suffix: "structural-only",
            },
            Self::LooselySimilar => KindLabels {
                title: "Similar code",
                taxonomy: "Type-3 clone with larger edits",
                css_suffix: "loosely-similar",
            },
        }
    }
}

/// [METRICS-TOTAL-INCLUDE] The kinds whose lines count toward the repo
/// total. Resolved once from `[total] include` (default:
/// [`DEFAULT_TOTAL_INCLUDES`]) and carried into the metric fold, so the
/// headline figure, every per-file and folder numerator, and the wire's
/// `total_includes` all read one set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotalInclude(Vec<ClusterKind>);

impl TotalInclude {
    /// Builds the set from `kinds`, deduplicated and sorted into the
    /// category display order so the wire spelling is deterministic.
    /// An empty set is legal — a user who opts every kind out gets a
    /// total of zero, not an error.
    #[must_use]
    pub fn from_kinds(kinds: Vec<ClusterKind>) -> Self {
        let mut kinds = kinds;
        kinds.sort_by_key(|kind| kind.display_position());
        kinds.dedup();
        Self(kinds)
    }

    /// True when `kind`'s lines count toward the total.
    #[must_use]
    pub fn contains(&self, kind: ClusterKind) -> bool {
        self.0.contains(&kind)
    }

    /// The included kinds, in category display order — the wire form.
    #[must_use]
    pub fn kinds(&self) -> &[ClusterKind] {
        &self.0
    }
}

impl Default for TotalInclude {
    fn default() -> Self {
        Self(DEFAULT_TOTAL_INCLUDES.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_listed_once_in_the_display_order() {
        let kinds = ClusterKind::all();
        // [CLONE-KIND-LABELS] The display order is fixed: Identical,
        // Nearly identical, Similar, Same behavior, shape-only last
        // ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
        assert_eq!(
            kinds,
            [
                ClusterKind::Identical,
                ClusterKind::NearlyIdentical,
                ClusterKind::LooselySimilar,
                ClusterKind::SameBehavior,
                ClusterKind::StructuralOnly,
            ]
        );
        for pair in kinds.windows(2) {
            let [earlier, later] = pair else {
                continue;
            };
            assert_ne!(earlier, later, "no kind is listed twice");
        }
        // The fold order is a separate concern and stays untouched
        // ([CLONE-KIND-FOLD]): display order is not strength order —
        // Similar (weaker) deliberately lists above Same behavior.
        assert!(
            ClusterKind::LooselySimilar.strength() < ClusterKind::SameBehavior.strength(),
            "strength remains the fold order, unchanged by display order"
        );
    }

    #[test]
    fn wire_labels_round_trip_and_include_set_normalizes() {
        for kind in ClusterKind::all() {
            assert_eq!(
                ClusterKind::from_wire_label(kind.wire_label()),
                Some(kind),
                "wire label round-trips for {kind:?}"
            );
        }
        assert_eq!(ClusterKind::from_wire_label("identical-ish"), None);
        // Default set, display order, and dedup all read from one place.
        assert_eq!(
            TotalInclude::default().kinds(),
            DEFAULT_TOTAL_INCLUDES.as_slice()
        );
        assert_eq!(
            TotalInclude::from_kinds(vec![
                ClusterKind::SameBehavior,
                ClusterKind::Identical,
                ClusterKind::Identical,
            ])
            .kinds(),
            &[ClusterKind::Identical, ClusterKind::SameBehavior]
        );
        assert_eq!(TotalInclude::from_kinds(Vec::new()).kinds(), []);
    }

    #[test]
    fn the_fold_keeps_the_weaker_kind_whichever_side_it_is_on() {
        assert_eq!(
            ClusterKind::Identical.weaker(ClusterKind::NearlyIdentical),
            ClusterKind::NearlyIdentical
        );
        assert_eq!(
            ClusterKind::NearlyIdentical.weaker(ClusterKind::Identical),
            ClusterKind::NearlyIdentical
        );
        assert_eq!(
            ClusterKind::SameBehavior.weaker(ClusterKind::StructuralOnly),
            ClusterKind::StructuralOnly
        );
        assert_eq!(
            ClusterKind::StructuralOnly.weaker(ClusterKind::LooselySimilar),
            ClusterKind::StructuralOnly
        );
        assert_eq!(
            ClusterKind::Identical.weaker(ClusterKind::Identical),
            ClusterKind::Identical
        );
    }

    #[test]
    fn a_rejected_pair_has_no_clone_kind() {
        assert_eq!(ClusterKind::from_pair(None), None);
        assert_eq!(
            ClusterKind::from_pair(Some(PairClassification::Identical)),
            Some(ClusterKind::Identical)
        );
        assert_eq!(
            ClusterKind::from_pair(Some(PairClassification::StructuralOnly)),
            Some(ClusterKind::StructuralOnly)
        );
    }

    #[test]
    fn wire_labels_match_the_serde_spelling_and_titles_carry_no_advice() -> anyhow::Result<()> {
        for kind in ClusterKind::all() {
            let serde_form = serde_json::to_value(kind)?;
            assert_eq!(serde_form, serde_json::Value::from(kind.wire_label()));
            let labels = kind.labels();
            assert!(
                !labels.title.contains("merge") && !labels.title.contains("safe"),
                "{kind:?} title advises instead of naming: {}",
                labels.title
            );
            assert!(
                !labels.title.contains("Type-"),
                "{kind:?} title carries jargon"
            );
            assert!(
                !labels.css_suffix.contains('_'),
                "{kind:?} css suffix is kebab-case"
            );
        }
        Ok(())
    }
}
