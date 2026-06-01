int accumulate(int bound) {
  if (bound < 0) {
    return 0;
  }
  var running = 0;
  for (var step = 0; step < bound; step = step + 1) {
    running = running + step;
    running = running + 2;
  }
  return running;
}
