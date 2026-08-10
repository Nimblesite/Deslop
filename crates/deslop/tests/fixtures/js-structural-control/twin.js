// Reduces a batch of orders into a single weighted score.
export function scoreShipments(orders) {
  let score = 0;
  for (const order of orders) {
    const mass = order.priority * order.volume;
    score = score + mass;
  }
  return score;
}
