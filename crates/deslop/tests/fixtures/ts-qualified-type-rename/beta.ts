export function formatLocal(clock: Temporal.Instant, value: number): string {
  const scaled = value * 2;
  return clock.format(scaled);
}
