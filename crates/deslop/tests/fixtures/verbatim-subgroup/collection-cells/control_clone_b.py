# The pasted copy of the control clone. The banner differs so the two
# files are not byte-identical; `settle_ledger` itself is untouched.


def settle_ledger(entries, floor, penalty):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 3 - penalty
        else:
            balance += entry // 2
    return balance
