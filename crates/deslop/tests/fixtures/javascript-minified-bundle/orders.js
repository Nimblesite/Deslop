// First-party source with a genuine duplicate of summarise() in invoices.js.
export function summariseOrders(rows) {
  let total = 0;
  let count = 0;
  for (const row of rows) {
    if (row.status === "settled") {
      total = total + row.amount;
      count = count + 1;
    }
  }
  return { total: total, count: count, average: count === 0 ? 0 : total / count };
}
