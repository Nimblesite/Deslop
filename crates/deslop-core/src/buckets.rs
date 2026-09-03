//! Pair-content support primitives retained for admission and explicit comparison.

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
