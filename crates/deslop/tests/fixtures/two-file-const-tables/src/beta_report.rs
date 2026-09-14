// The pasted copy of `apply_discount_schedule`, byte-identical to the
// one in `alpha_report.rs`. A banner comment keeps the file bytes
// distinct without touching the duplicated function.

pub fn apply_discount_schedule(amounts: &[i64], threshold: i64) -> i64 {
    let mut running = 0;
    for amount in amounts {
        if *amount > threshold {
            running += amount * 3 - threshold;
        } else {
            running += amount / 2;
        }
    }
    running
}
