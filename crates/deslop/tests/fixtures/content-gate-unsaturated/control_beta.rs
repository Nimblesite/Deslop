//! Byte-identical control half B.

/// Sums the weighted scores of a slice.
pub fn weighted_total(scores: &[i64], weight: i64) -> i64 {
    let mut total = 0;
    for score in scores {
        total += score * weight;
    }
    total
}
