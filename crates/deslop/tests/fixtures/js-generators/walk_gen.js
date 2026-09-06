export function* iterate(start, limit, delta) {
  let current = start;
  while (current < limit) {
    yield current;
    yield current * 2;
    current = current + delta;
  }
  return current;
}
