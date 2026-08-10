export function route(weight: number, distance: number, carrier: string): string {
  const score = weight * 3 + distance;
  if (score > 900) {
    return carrier + "-freight";
  }
  if (score > 400) {
    return carrier + "-ground";
  }
  return carrier + "-parcel";
}
