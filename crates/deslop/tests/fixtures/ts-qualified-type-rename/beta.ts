export function formatZoned(stamp: Temporal.Instant, amount: number): string {
  const scaled = amount * 2;
  return stamp.format(scaled);
}
