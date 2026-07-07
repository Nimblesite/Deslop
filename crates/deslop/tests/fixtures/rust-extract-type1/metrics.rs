pub fn total_with_tax(amounts: &[usize], tax_rate: usize) -> usize {
    let mut total = 0;
    for amount in amounts {
        let taxed = amount * tax_rate / 100;
        total += amount + taxed;
    }
    if total > 10_000 {
        total = 10_000;
    }
    total
}

pub fn subtotal_with_tax(amounts: &[usize], tax_rate: usize) -> usize {
    let mut total = 0;
    for amount in amounts {
        let taxed = amount * tax_rate / 100;
        total += amount + taxed;
    }
    if total > 10_000 {
        total = 10_000;
    }
    total
}
