//! Iterator-chain twin of `aggregates_loop.rs`. Same five functions,
//! same semantics, entirely different AST.

pub fn sum(values: &[i64]) -> i64 {
    values.iter().copied().fold(0_i64, i64::saturating_add)
}

pub fn product(values: &[i64]) -> i64 {
    if values.is_empty() {
        0
    } else {
        values.iter().copied().fold(1_i64, i64::saturating_mul)
    }
}

pub fn count_positive(values: &[i64]) -> usize {
    values.iter().filter(|value| **value > 0).count()
}

pub fn max_value(values: &[i64]) -> Option<i64> {
    values.iter().copied().max()
}

pub fn running_total(values: &[i64]) -> Vec<i64> {
    values
        .iter()
        .scan(0_i64, |running, value| {
            *running = running.saturating_add(*value);
            Some(*running)
        })
        .collect()
}
