int summarize(List<int> values) {
  var total = 0;
  for (final value in values) {
    total = total + value;
  }
  return total;
}
