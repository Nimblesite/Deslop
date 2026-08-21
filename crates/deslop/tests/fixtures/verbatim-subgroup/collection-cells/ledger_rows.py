values = [
    normalize(invoice.amount, invoice.currency, invoice.rate),
    normalize(invoice.amount, invoice.currency, invoice.rate),
    normalize(refund.total, refund.symbol, refund.factor),
]
