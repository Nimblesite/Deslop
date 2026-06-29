export function formatLocal(clock: Intl.DateTimeFormat, value: number): string {
  const scaled = value * 2;
  return clock.format(scaled);
}
