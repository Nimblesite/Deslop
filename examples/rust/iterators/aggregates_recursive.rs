//! Recursive twin of `aggregates_loop.rs` / `aggregates_iter.rs`.
//! Same five functions re-expressed via tail recursion over a slice
//! prefix. Pure Type-4 with respect to the other two files.

pub fn sum(values: &[i64]) -> i64 {
    match values.split_first() {
        None => 0,
        Some((head, tail)) => head.saturating_add(sum(tail)),
    }
}

pub fn product(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    product_inner(values)
}

fn product_inner(values: &[i64]) -> i64 {
    match values.split_first() {
        None => 1,
        Some((head, tail)) => head.saturating_mul(product_inner(tail)),
    }
}

pub fn count_positive(values: &[i64]) -> usize {
    match values.split_first() {
        None => 0,
        Some((head, tail)) => {
            let here = if *head > 0 { 1 } else { 0 };
            here + count_positive(tail)
        }
    }
}

pub fn max_value(values: &[i64]) -> Option<i64> {
    match values.split_first() {
        None => None,
        Some((head, tail)) => match max_value(tail) {
            None => Some(*head),
            Some(rest_max) => Some(if *head > rest_max { *head } else { rest_max }),
        },
    }
}

pub fn running_total(values: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(values.len());
    running_total_into(values, 0_i64, &mut out);
    out
}

fn running_total_into(values: &[i64], accumulator: i64, out: &mut Vec<i64>) {
    if let Some((head, tail)) = values.split_first() {
        let next = accumulator.saturating_add(*head);
        out.push(next);
        running_total_into(tail, next, out);
    }
}
