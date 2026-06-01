int run(int limit) {
  if (limit < 0) {
    return 0;
  }
  var accumulator = 0;
  for (var position = 0; position < limit; position = position + 1) {
    accumulator = accumulator + position;
  }
  return accumulator;
}
