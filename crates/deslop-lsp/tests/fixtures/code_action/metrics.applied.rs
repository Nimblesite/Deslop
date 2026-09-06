// TODO: deslop — replace `DeslopTodo` with real types.
type DeslopTodo = ();

fn extracted_from_cluster_d071d5(amounts: DeslopTodo, tax_rate: DeslopTodo) -> DeslopTodo {
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

pub fn total_with_tax(amounts: &[usize], tax_rate: usize) -> usize {
    extracted_from_cluster_d071d5(amounts, tax_rate)
}

pub fn subtotal_with_tax(amounts: &[usize], tax_rate: usize) -> usize {
    extracted_from_cluster_d071d5(amounts, tax_rate)
}
