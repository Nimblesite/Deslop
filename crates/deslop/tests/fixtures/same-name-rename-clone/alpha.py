def summarise_ledger(rows, rate, cut):
    total = 0.0
    for item in rows:
        if item.skipped:
            continue
        line = item.quantity * item.unit_price
        if item.quantity > 10:
            line = line * (1.0 - cut)
        total = total + line
    charge = total * rate
    gross = total + charge
    return {
        "subtotal": round(total, 2),
        "tax": round(charge, 2),
        "total": round(gross, 2),
    }
