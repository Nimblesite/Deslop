def total_with_tax(amounts, tax_rate):
    total = 0
    for amount in amounts:
        taxed = amount * tax_rate // 100
        total = total + amount + taxed
    if total > 10000:
        total = 10000
    return total


def subtotal_with_tax(amounts, tax_rate):
    total = 0
    for amount in amounts:
        taxed = amount * tax_rate // 100
        total = total + amount + taxed
    if total > 10000:
        total = 10000
    return total
