//! The clone-kind registry ([CLONE-KIND-LABELS]): one label set per
//! [`ClusterKind`], the strength order the cluster fold reads
//! ([CLONE-KIND-FOLD]), and the pair-content support floors that
//! admission and explicit comparison share ([FUSED-CONTENT-GATE]).

pub use crate::wire_generated::ClusterKind;
use crate::wire_generated::PairClassification;
pub(crate) mod grouping;

/// Default pair-content support floor ([FUSED-CONTENT-GATE]).
pub const CONTENT_SUPPORT_FLOOR: f64 = 0.7;

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

    /// Every kind, strongest first — the order surfaces list kinds in.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Identical,
            Self::NearlyIdentical,
            Self::SameBehavior,
            Self::LooselySimilar,
            Self::StructuralOnly,
        ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_listed_once_strongest_first() {
        let kinds = ClusterKind::all();
        for pair in kinds.windows(2) {
            let [stronger, weaker] = pair else {
                continue;
            };
            assert!(
                stronger.strength() > weaker.strength(),
                "{stronger:?} must list before {weaker:?}"
            );
        }
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
