int aggregate(int limit) {
  if (limit < 0) {
    return 0;
  }
  var accumulator = 0;
  for (var cursor = 0; cursor < limit; cursor = cursor + 1) {
    accumulator = accumulator + cursor;
  }
  return accumulator;
}
