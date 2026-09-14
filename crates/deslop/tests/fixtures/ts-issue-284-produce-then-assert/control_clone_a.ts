// False-negative control: `settleLedger` below is copy-pasted
// byte-for-byte into `control_clone_b.ts`. Whatever hides the object
// literals must leave this pair visible.

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
