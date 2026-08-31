// ledger_beta.ts — the pasted copy of the TypeScript reconciliation routine.

export const LEDGER_TAG = "ledger";

export function reconcileEntries(entries: number[], floor: number): number {
  let balance = 0;
  for (const entry of entries) {
    if (entry > floor) {
      balance += entry * 2;
    } else {
      balance -= Math.trunc(entry / 2);
    }
  }
  return balance;
}
