export interface Invoice {
  lines: Array<{ cost: number; count: number }>;
}

export async function collectInvoices(invoices: Invoice[]): Promise<number[]> {
  const amounts = await Promise.all(
    invoices.map(async (invoice) => {
      const gross = invoice.lines.reduce(
        (sum, line) => sum + line.cost * line.count,
        0,
      );
      return gross > 100 ? gross * 0.9 : gross;
    }),
  );
  return amounts.filter((amount) => amount > 0);
}
