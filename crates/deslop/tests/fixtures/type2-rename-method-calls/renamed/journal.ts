export function buildJournalReport(movements: Movement[], cap: number): Journal {
  const journal = new Journal();
  let accrued = 0;
  for (const movement of movements) {
    const value = movement.value;
    if (value > cap) {
      journal.dropRecord(movement.id);
      continue;
    }
    accrued = accrued + value;
    journal.writeRecord(movement.id, value);
    if (accrued > cap) {
      journal.writeRecord(movement.id, cap);
      journal.dropRecord(movement.id);
      accrued = cap;
    }
  }
  journal.writeRecord("total", accrued);
  return journal;
}
