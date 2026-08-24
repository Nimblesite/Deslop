export function accumulate(values: number[], floor: number): number {
  let total = 0;
  for (const value of values) {
    if (value > floor) {
      total = total + value * 2;
    } else {
      total = total - 1;
    }
  }
  return total;
}
