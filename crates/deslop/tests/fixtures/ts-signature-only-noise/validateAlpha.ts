export function validateAlpha(context: Context, options: Options): Outcome {
  return { ok: true, value: context.read(options.key) };
}
