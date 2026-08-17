// ledger_alpha.ts — the canonical copy of the TypeScript reconciliation routine.

export const ALPHA_LEDGER_TAG = "alpha";

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
