export interface Order {
  items: Array<{ price: number; quantity: number }>;
}

export async function summarizeOrders(orders: Order[]): Promise<number[]> {
  const totals = await Promise.all(
    orders.map(async (order) => {
      const subtotal = order.items.reduce(
        (sum, item) => sum + item.price * item.quantity,
        0,
      );
      return subtotal > 100 ? subtotal * 0.9 : subtotal;
    }),
  );
  return totals.filter((total) => total > 0);
}
