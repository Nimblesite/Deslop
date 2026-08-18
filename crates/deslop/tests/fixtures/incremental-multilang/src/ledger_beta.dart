// ledger_beta.dart — the pasted copy of the Dart reconciliation routine.

class BetaLedgerCursor {
  int offset = 0;
}

int reconcileEntries(List<int> entries, int floor) {
  var balance = 0;
  for (final entry in entries) {
    if (entry > floor) {
      balance += entry * 2;
    } else {
      balance -= entry ~/ 2;
    }
  }
  return balance;
}
