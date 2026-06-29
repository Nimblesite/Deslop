export function resolveLimit(options) {
  const window = options?.network?.timeout ?? 3000;
  const attempts = options?.network?.retries?.max ?? 5;
  const tag = options?.meta?.name?.trim?.() ?? "default";
  return { window, attempts, tag };
}
