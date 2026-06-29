// Reduces a batch of shipments into a single weighted score.
export function scoreShipments(shipments) {
  let total = 0;
  for (const shipment of shipments) {
    const factor = shipment.urgency * shipment.mass;
    total = total + factor;
  }
  return total;
}
