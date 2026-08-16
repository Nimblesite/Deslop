//! Beta billing module. Carries occurrence 2 of 3 of the
//! `settle_invoice` clone (the larger cluster in the golden report).

pub struct BetaMarker;

pub fn settle_invoice(amounts: &[i64], tax_rate: i64) -> i64 {
    let mut total = 0;
    for amount in amounts {
        let taxed = amount * tax_rate / 100;
        if *amount > 0 {
            total += amount + taxed;
        } else {
            total -= taxed;
        }
    }
    if total < 0 {
        total = 0;
    }
    total
}
