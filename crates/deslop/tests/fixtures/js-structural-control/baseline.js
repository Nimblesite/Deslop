/* Reduces a batch of orders into a single weighted score. */
export function scoreBatch(orders) {
  let score = 0;
  for (const order of orders) {
    const weight = order.priority * order.volume;
    score = score + weight;
  }
  return score;
}
