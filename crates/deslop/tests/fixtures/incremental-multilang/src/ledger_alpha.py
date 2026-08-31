# ledger_alpha.py — the canonical copy of the Python reconciliation routine.

LEDGER_TAG = "ledger"


def reconcile_entries(entries, floor):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 2
        else:
            balance -= entry // 2
    return balance
