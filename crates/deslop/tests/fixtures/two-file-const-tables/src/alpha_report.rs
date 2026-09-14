// The genuine duplication in this fixture: `apply_discount_schedule` is
// byte-identical to the copy in `beta_report.rs`. It is the
// false-negative control — whatever suppresses the const tables must
// leave this visible and ranked first.

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
