export function computeInvoiceTotal(lineItems: number[], taxRate: number): number {
  let runningTotal = 0;
  for (const lineAmount of lineItems) {
    runningTotal = runningTotal + lineAmount;
  }
  const taxDue = runningTotal * taxRate;
  const grandTotal = runningTotal + taxDue;
  return grandTotal;
}
