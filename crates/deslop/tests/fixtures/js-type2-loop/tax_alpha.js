export function computeLineTotals(lineItems, taxRate) {
  const totals = [];
  for (let index = 0; index < lineItems.length; index += 1) {
    const lineItem = lineItems[index];
    let amount = lineItem.unitPrice * lineItem.quantity;
    if (lineItem.taxable) {
      amount = amount + amount * taxRate;
    }
    if (amount > 0) {
      totals.push({ sku: lineItem.sku, amount: amount });
    }
  }
  return totals;
}
