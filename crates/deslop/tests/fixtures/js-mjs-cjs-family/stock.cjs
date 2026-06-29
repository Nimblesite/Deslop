const { clampLow } = require("./bounds.cjs");

function reconcileInventory(records) {
  const adjustments = [];
  for (const record of records) {
    let balance = record.opening;
    for (const movement of record.movements) {
      if (movement.kind === "inbound") {
        balance += movement.quantity;
      } else {
        balance -= movement.quantity;
      }
    }
    adjustments.push({ sku: record.sku, balance });
  }
  return adjustments.filter((entry) => clampLow(entry.balance) >= 0);
}

module.exports = { reconcileInventory };
