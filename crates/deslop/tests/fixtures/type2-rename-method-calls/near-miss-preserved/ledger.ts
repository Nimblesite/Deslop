export function buildInvoiceReport(entries: Entry[], limit: number): Ledger {
  const ledger = new Ledger();
  let carried = 0;
  for (const entry of entries) {
    const amount = entry.amount;
    if (amount > limit) {
      ledger.skipEntry(entry.id);
      continue;
    }
    carried = carried + amount;
    ledger.postEntry(entry.id, amount);
    if (carried > limit) {
      ledger.postEntry(entry.id, limit);
      ledger.skipEntry(entry.id);
      carried = limit;
    }
  }
  ledger.postEntry("total", carried);
  return ledger;
}
