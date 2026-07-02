export function aggregate(items: number[]): number {
  let accumulator = 0;
  for (const cursor of items) {
    accumulator = accumulator + cursor;
    if (accumulator > 50) {
      return accumulator;
    }
  }
  return accumulator;
}
