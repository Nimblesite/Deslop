export function buildInvoiceReport(entries: Entry[], limit: number): Ledger {
  const ledger = new Ledger();
  let carried = 0;
  for (const entry of entries) {
    const amount = entry.amount;
    if (amount > limit) {
      ledger.holdEntry(entry.id);
      continue;
    }
    const previous = carried;
    carried = carried + amount;
    ledger.voidEntry(entry.id, amount);
    if (carried > limit) {
      ledger.voidEntry(entry.id, limit);
      ledger.holdEntry(entry.id);
      carried = limit;
    }
  }
  ledger.voidEntry("total", carried);
  return ledger;
}
