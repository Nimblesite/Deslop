int totalRecursive(int limit) {
  if (limit <= 0) {
    return 0;
  }
  return limit + totalRecursive(limit - 1);
}

int totalIterative(int limit) {
  var running = 0;
  var index = 1;
  while (index <= limit) {
    running = running + index;
    index = index + 1;
  }
  return running;
}
