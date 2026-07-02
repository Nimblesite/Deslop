export function scanStock(entries) {
  const alerts = [];
  for (const entry of entries) {
    if (entry.discontinued) {
      continue;
    }
    const available = entry.onHand - entry.reserved;
    if (available < entry.reorderPoint) {
      alerts.push({ sku: entry.sku, deficit: entry.reorderPoint - available });
    }
  }
  return alerts;
}
