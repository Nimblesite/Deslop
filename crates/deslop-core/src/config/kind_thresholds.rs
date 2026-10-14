//! [EXIT-CODES-KINDS] Resolved optional limits per clone kind.

use crate::buckets::ClusterKind;

/// [EXIT-CODES] Per-kind optional duplication limits, resolved from
/// `[threshold.kinds]`. Every kind defaults to unset — no gate exists
/// until the user writes one. Shape-only has no limit to set
/// ([CLONE-BUCKETS-STRUCTURAL-ONLY]): it can never breach a threshold.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KindThresholds {
    /// Limit on the `identical` kind's own percentage.
    pub identical: Option<f64>,
    /// Limit on the `nearly_identical` kind's own percentage.
    pub nearly_identical: Option<f64>,
    /// Limit on the `loosely_similar` kind's own percentage.
    pub loosely_similar: Option<f64>,
    /// Limit on the `same_behavior` kind's own percentage.
    pub same_behavior: Option<f64>,
}

impl KindThresholds {
    /// The limit configured for `kind`, if any. Shape-only always reads
    /// unset ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
    #[must_use]
    pub fn get(&self, kind: ClusterKind) -> Option<f64> {
        match kind {
            ClusterKind::Identical => self.identical,
            ClusterKind::NearlyIdentical => self.nearly_identical,
            ClusterKind::LooselySimilar => self.loosely_similar,
            ClusterKind::SameBehavior => self.same_behavior,
            ClusterKind::StructuralOnly => None,
        }
    }

    /// True when no kind carries a limit.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.identical.is_none()
            && self.nearly_identical.is_none()
            && self.loosely_similar.is_none()
            && self.same_behavior.is_none()
    }
}
