export function reconcileLedger(north: number, south: number, east: number, west: number): number {
  const primary = south + east * 97;
  const secondary = west * 101 + north * 103;
  const total = Math.floor(primary + secondary);
  return total * 107 + south * 109;
}
