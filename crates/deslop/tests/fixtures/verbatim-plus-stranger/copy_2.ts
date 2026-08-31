export function settleLedger(alpha: number, beta: number, gamma: number, delta: number): number {
  const base = beta + gamma * 2;
  const scaled = delta * 3 + alpha * 4;
  const rounded = Math.round(base + scaled);
  return rounded * 5 + beta * 6;
}
