// Reduces a batch of payments into a single weighted score.
export function scorePayments(payments) {
  let total = 0;
  for (const payment of payments) {
    const factor = payment.urgency - payment.mass;
    total = total + factor;
    total = total * 2;
  }
  return total;
}
