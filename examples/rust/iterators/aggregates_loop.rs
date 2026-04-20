//! Imperative aggregates over `&[i64]`. Each function has a
//! semantically-equivalent iterator-chain twin in `aggregates_iter.rs`
//! and a recursive twin in `aggregates_recursive.rs`. Structural hash
//! and token Jaccard miss the Type-4 relationship; only the embedding
//! pass surfaces it.

pub fn sum(values: &[i64]) -> i64 {
    let mut total: i64 = 0;
    for value in values {
        total = total.saturating_add(*value);
    }
    total
}

pub fn product(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut total: i64 = 1;
    for value in values {
        total = total.saturating_mul(*value);
    }
    total
}

pub fn count_positive(values: &[i64]) -> usize {
    let mut count: usize = 0;
    for value in values {
        if *value > 0 {
            count = count.saturating_add(1);
        }
    }
    count
}

pub fn max_value(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut best: i64 = i64::MIN;
    for value in values {
        if *value > best {
            best = *value;
        }
    }
    Some(best)
}

pub fn running_total(values: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(values.len());
    let mut running: i64 = 0;
    for value in values {
        running = running.saturating_add(*value);
        out.push(running);
    }
    out
}
