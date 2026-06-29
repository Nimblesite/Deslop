/** Domain shape for a customer order with priced line items. */
export interface Order {
  items: Array<{ price: number; quantity: number }>;
}

/** Aggregates each order into a discounted total and keeps the positives. */
export async function summariseOrders(orders: Order[]): Promise<number[]> {
  const totals = await Promise.all(
    orders.map(async (order) => {
      const subtotal = order.items.reduce(
        (running, item) => running + item.price * item.quantity,
        0,
      );
      const tier = subtotal > 100 ? "gold" : "silver";
      return tier === "gold" ? subtotal * 0.9 : subtotal;
    }),
  );
  return totals.filter((total) => total > 0);
}
