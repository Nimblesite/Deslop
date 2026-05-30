int compute(int input) {
  if (input < 0) {
    return 0;
  }
  var total = 0;
  for (var index = 0; index < input; index = index + 1) {
    total = total + index;
  }
  return total;
}
