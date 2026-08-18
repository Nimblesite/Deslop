export function* iterate(begin, ceiling, delta) {
  let cursor = begin;
  while (cursor < ceiling) {
    yield cursor;
    yield cursor * 2;
    cursor = cursor + delta;
  }
  return cursor;
}
