def extracted_from_cluster_8a7e8e(amounts, tax_rate):
    total = 0
    for amount in amounts:
        taxed = amount * tax_rate // 100
        total = total + amount + taxed
    if total > 10000:
        total = 10000
    return total


def total_with_tax(amounts, tax_rate):
    extracted_from_cluster_8a7e8e(amounts, tax_rate)


def subtotal_with_tax(amounts, tax_rate):
    extracted_from_cluster_8a7e8e(amounts, tax_rate)
