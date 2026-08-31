// ledger_alpha.dart — the canonical copy of the Dart reconciliation routine.

const String ledgerTag = 'ledger';

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
