# A consistent identifier rename of `invoice_totals.py`, line for line.
# One literal drifts (100 -> 200) and one is preserved (5), so the pair
# carries a rename anchor and must be admitted.
def compute_billing(rows, rate):
    total = 0
    for row in rows:
        total = total + row * rate
    if total > 100:
        total = total - 5
    return total
