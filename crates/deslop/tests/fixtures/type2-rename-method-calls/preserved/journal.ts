export function buildJournalReport(movements: Movement[], cap: number): Journal {
  const journal = new Journal();
  let accrued = 0;
  for (const movement of movements) {
    const value = movement.value;
    if (value > cap) {
      journal.skipEntry(movement.id);
      continue;
    }
    accrued = accrued + value;
    journal.postEntry(movement.id, value);
    if (accrued > cap) {
      journal.postEntry(movement.id, cap);
      journal.skipEntry(movement.id);
      accrued = cap;
    }
  }
  journal.postEntry("total", accrued);
  return journal;
}
