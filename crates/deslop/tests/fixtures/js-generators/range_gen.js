export function* countUp(start, limit, step) {
  let current = start;
  while (current < limit) {
    yield current;
    yield current * 2;
    current = current + step;
  }
  return current;
}
