const INITIAL_TOTAL: i64 = 0;

pub fn compute_charge(entries: &[i64], rate: i64) -> i64 {
    entries.iter().fold(INITIAL_TOTAL, |total, entry| {
        let scaled = entry.saturating_mul(rate);
        let adjusted = if scaled.is_positive() {
            scaled.saturating_add(rate)
        } else {
            scaled.saturating_sub(rate)
        };
        total.saturating_add(adjusted)
    })
}
