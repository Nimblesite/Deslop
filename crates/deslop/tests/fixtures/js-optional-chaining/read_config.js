export function resolveTimeout(config) {
  const network = config?.network?.timeout ?? 3000;
  const retries = config?.network?.retries?.max ?? 5;
  const label = config?.meta?.name?.trim?.() ?? "default";
  return { network, retries, label };
}
