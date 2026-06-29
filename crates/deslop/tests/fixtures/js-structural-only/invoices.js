export function summarizeInvoices(invoices) {
  return invoices
    .map((invoice) =>
      invoice.lines.reduce(
        (grossSoFar, line) =>
          grossSoFar + line.rate * line.hours * (1 - line.deduction),
        0,
      ),
    )
    .filter((invoiceTotal) => invoiceTotal > 50)
    .map((invoiceTotal) => Math.round(invoiceTotal * 100) / 100);
}
