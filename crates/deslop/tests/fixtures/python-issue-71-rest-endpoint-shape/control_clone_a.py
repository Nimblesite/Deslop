# False-negative control for this fixture. `settle_ledger` below is
# copy-pasted byte-for-byte into `control_clone_b.py`. Whatever hides
# the noise family in this directory must leave this pair visible — a
# suppression test that only asserts an empty report passes just as well
# when the detector has gone blind.


def settle_ledger(entries, floor, penalty):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 3 - penalty
        else:
            balance += entry // 2
    return balance
