export function summarizeOrders(orders) {
  return orders
    .map((order) =>
      order.items.reduce(
        (runningTotal, item) =>
          runningTotal + item.price * item.quantity * (1 - item.discount),
        0,
      ),
    )
    .filter((orderTotal) => orderTotal > 50)
    .map((orderTotal) => Math.round(orderTotal * 100) / 100);
}
