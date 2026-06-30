export function accumulate(values) {
  let running = 0;
  for (const step of values) {
    running = running + step;
    running = running + 2;
    if (running > 50) {
      return running;
    }
  }
  return running;
}
