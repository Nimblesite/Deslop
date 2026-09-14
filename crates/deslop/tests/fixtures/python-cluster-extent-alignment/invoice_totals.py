# The renamed twin of `billing_totals.py`. Every reported occurrence of
# the pair must cover the same authored declaration: a view that opens at
# the `def` line in one file and inside the body in the other is not one
# duplication ([PIPELINE-CLUSTER-EXACT-SCOPE]).
def compute_invoice(items, factor):
    amount = 0
    for item in items:
        amount = amount + item * factor
    if amount > 200:
        amount = amount - 5
    return amount
