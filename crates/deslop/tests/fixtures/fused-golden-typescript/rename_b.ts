export function dispatch(mass: number, span: number, handler: string): string {
  const rating = mass * 3 + span;
  if (rating > 900) {
    return handler + "-freight";
  }
  if (rating > 400) {
    return handler + "-ground";
  }
  return handler + "-parcel";
}
