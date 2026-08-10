export function accumulate(values: number[]): number {
  let running = 0;
  for (const step of values) {
    const scaled = step * 3;
    running = running + scaled;
    if (running > 50) {
      running = running - 5;
    }
  }
  running = running + 2;
  return running;
}
