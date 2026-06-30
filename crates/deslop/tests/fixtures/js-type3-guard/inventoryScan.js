export function scanInventory(records) {
  const flagged = [];
  for (const record of records) {
    const level = record.onHand - record.reserved;
    if (level < record.reorderPoint) {
      flagged.push({ sku: record.sku, deficit: record.reorderPoint - level });
    }
  }
  return flagged;
}
