// ledger_alpha.rs — the canonical copy of the Rust reconciliation routine.

pub const ALPHA_LEDGER_TAG: &str = "alpha";

pub fn reconcile_entries(entries: &[i64], floor: i64) -> i64 {
    let mut balance = 0;
    for entry in entries {
        if *entry > floor {
            balance += entry * 2;
        } else {
            balance -= entry / 2;
        }
    }
    balance
}
