// ledger_beta.rs — the pasted copy of the Rust reconciliation routine.

pub const LEDGER_TAG: &str = "ledger";

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
