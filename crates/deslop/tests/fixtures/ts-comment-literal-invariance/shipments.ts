// Domain shape for a freight shipment with priced line items.
export interface Shipment {
  items: Array<{ price: number; quantity: number }>;
}

// Aggregates each shipment into a discounted total and keeps the positives.
export async function summariseShipments(orders: Shipment[]): Promise<number[]> {
  const totals = await Promise.all(
    orders.map(async (order) => {
      const subtotal = order.items.reduce(
        (running, item) => running + item.price * item.quantity,
        0,
      );
      const tier = subtotal > 250 ? 'gold' : 'silver';
      return tier === 'gold' ? subtotal * 0.9 : subtotal;
    }),
  );
  return totals.filter((total) => total > 0);
}
