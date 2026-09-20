//! [CLONE-BUCKETS-THRESHOLDS] Validated category thresholds from `.deslop.toml`.

use std::path::Path;

use serde::Deserialize;

use crate::error::CoreError;

/// Default minimum structure similarity for a general near-copy.
pub const DEFAULT_NEARLY_IDENTICAL_MIN_SHAPE: f64 = 0.90;
/// Default minimum content support for a general near-copy.
pub const DEFAULT_NEARLY_IDENTICAL_MIN_CONTENT: f64 = 0.70;
/// Default minimum content support for a general Similar finding.
pub const DEFAULT_SIMILAR_MIN_CONTENT: f64 = 0.50;
/// Default ceiling for negligible shared content.
pub const DEFAULT_SHAPE_ONLY_MAX_CONTENT: f64 = 0.05;

pub use crate::wire_generated::RoutingTuning;

impl Default for RoutingTuning {
    fn default() -> Self {
        Self {
            nearly_identical_min_shape: DEFAULT_NEARLY_IDENTICAL_MIN_SHAPE,
            nearly_identical_min_content: DEFAULT_NEARLY_IDENTICAL_MIN_CONTENT,
            similar_min_content: DEFAULT_SIMILAR_MIN_CONTENT,
            shape_only_max_content: DEFAULT_SHAPE_ONLY_MAX_CONTENT,
        }
    }
}

impl RoutingTuning {
    /// Rejects impossible ranges and overlapping category boundaries.
    pub(super) fn validate(self, path: &Path) -> Result<Self, CoreError> {
        for (name, value) in self.values() {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(invalid(
                    path,
                    &format!("tuning.routing.{name} must be finite and within [0, 1]"),
                ));
            }
        }
        if self.shape_only_max_content >= self.similar_min_content
            || self.similar_min_content > self.nearly_identical_min_content
        {
            return Err(invalid(path, "tuning.routing requires shape_only_max_content < similar_min_content <= nearly_identical_min_content"));
        }
        Ok(self)
    }

    /// Named effective values for validation and report metadata.
    #[must_use]
    pub const fn values(self) -> [(&'static str, f64); 4] {
        [
            (
                "nearly_identical_min_shape",
                self.nearly_identical_min_shape,
            ),
            (
                "nearly_identical_min_content",
                self.nearly_identical_min_content,
            ),
            ("similar_min_content", self.similar_min_content),
            ("shape_only_max_content", self.shape_only_max_content),
        ]
    }
}

/// Raw category-tuning section; defaults are owned by `RoutingTuning`.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Tuning {
    /// Category routing settings.
    pub(super) routing: RoutingTuning,
}

/// A configuration error tied to the file the user can correct.
fn invalid(path: &Path, message: &str) -> CoreError {
    CoreError::ConfigThreshold {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}
