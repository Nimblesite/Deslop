# ledger_alpha.py — the canonical copy of the Python reconciliation routine.

ALPHA_LEDGER_TAG = "alpha"


def reconcile_entries(entries, floor):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 2
        else:
            balance -= entry // 2
    return balance
