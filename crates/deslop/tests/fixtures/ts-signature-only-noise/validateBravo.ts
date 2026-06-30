export function validateBravo(scope: Context, options: Options): Outcome {
  for (const candidate of scope.entries()) {
    if (candidate.expired) {
      throw new Error("stale");
    }
  }
  return { ok: false, value: 0 };
}
