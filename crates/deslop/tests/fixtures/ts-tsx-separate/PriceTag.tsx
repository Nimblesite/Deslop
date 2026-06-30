export interface LineItem {
  unitPrice: number;
  quantity: number;
  taxable: boolean;
}

export function computeTotals(items: LineItem[]): number {
  let running = 0;
  for (const item of items) {
    const gross = item.unitPrice * item.quantity;
    running += item.taxable ? gross * 1.1 : gross;
  }
  return Math.round(running * 100) / 100;
}

export function PriceTag({ items }: { items: LineItem[] }) {
  return <strong className="price">{computeTotals(items)}</strong>;
}
