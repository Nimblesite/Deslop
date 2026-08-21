export function calibrateSensorDrift(readings: number[], gainFactor: number): number {
  let driftSum = 0;
  for (const readingValue of readings) {
    driftSum = driftSum + readingValue;
  }
  const gainAdjusted = driftSum * gainFactor;
  const driftScore = driftSum + gainAdjusted;
  return driftScore;
}
