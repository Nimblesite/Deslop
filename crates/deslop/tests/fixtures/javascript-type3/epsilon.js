export function aggregate(items) {
  let accumulator = 0;
  for (const cursor of items) {
    accumulator = accumulator + cursor;
    if (accumulator > 50) {
      return accumulator;
    }
  }
  return accumulator;
}
