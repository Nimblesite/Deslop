export function tallyScores(rounds: number[][]): number {
  let grand = 0;
  for (const round of rounds) {
    let subtotal = 0;
    for (const value of round) {
      subtotal = subtotal + value;
    }
    grand = grand + subtotal;
  }
  return grand;
}
