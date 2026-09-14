def summarise_ledger(entries, levy, discount):
    running = 0.0
    for item in entries:
        if item.voided:
            continue
        amount = item.count * item.price
        if item.count > 10:
            amount = amount * (1.0 - discount)
        running = running + amount
    charge = running * levy
    gross = running + charge
    return {
        "subtotal": round(running, 2),
        "tax": round(charge, 2),
        "total": round(gross, 2),
    }
