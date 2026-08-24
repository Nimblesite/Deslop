export function aggregate(items) {
  let running = 0;
  for (const step of items) {
    const scaled = step * 3;
    running = running + scaled;
    if (running > 50) {
      running = running - 5;
    }
  }
  return running;
}
