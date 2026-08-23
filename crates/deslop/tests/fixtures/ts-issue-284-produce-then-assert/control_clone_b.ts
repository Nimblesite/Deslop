// The pasted copy of the control clone. Only this banner differs.

export function settleLedger(entries: number[], floor: number, penalty: number): number {
  let balance = 0;
  for (const entry of entries) {
    if (entry > floor) {
      balance += entry * 3 - penalty;
    } else {
      balance += Math.floor(entry / 2);
    }
  }
  return balance;
}
