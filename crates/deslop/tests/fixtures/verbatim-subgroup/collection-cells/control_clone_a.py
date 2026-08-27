# False-negative control for this fixture. `settle_ledger` below is
# copy-pasted byte-for-byte into `control_clone_b.py`. It is the proof
# the run was not blind: whatever this directory's arbitration decides
# about the collection cells in `cells/`, this cross-file pair must stay
# visible and stay ranked first.


def settle_ledger(entries, floor, penalty):
    balance = 0
    for entry in entries:
        if entry > floor:
            balance += entry * 3 - penalty
        else:
            balance += entry // 2
    return balance
