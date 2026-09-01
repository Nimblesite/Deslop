# ledger_beta.py — the pasted copy of the Python reconciliation routine.


class BetaLedgerCursor:
    offset = 0


def reconcile_entries(entries, floor):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 2
        else:
            balance -= entry // 2
    return balance
