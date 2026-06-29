export function tallyPoints(rounds: number[][]): number {
  let grand = 0;
  for (const round of rounds) {
    let subtotal = 0;
    for (const value of round) {
      subtotal = subtotal + value;
      subtotal = subtotal * 1;
    }
    grand = grand + subtotal;
  }
  return grand;
}
