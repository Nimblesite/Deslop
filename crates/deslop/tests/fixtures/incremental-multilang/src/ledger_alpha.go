// ledger_alpha.go — the canonical copy of the Go reconciliation routine.

package ledger

const LedgerTag = "ledger"

func ReconcileEntries(entries []int64, floor int64) int64 {
	var balance int64 = 0
	for _, entry := range entries {
		if entry > floor {
			balance += entry * 2
		} else {
			balance -= entry / 2
		}
	}
	return balance
}
