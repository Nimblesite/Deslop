// Domain shape for a freight shipment with weighed parcels.
export interface Shipment {
  parcels: Array<{ weight: number; units: number }>;
}

// Aggregates each shipment into a discounted total and keeps the positives.
export async function summariseShipments(shipments: Shipment[]): Promise<number[]> {
  const grands = await Promise.all(
    shipments.map(async (shipment) => {
      const partial = shipment.parcels.reduce(
        (acc, parcel) => acc + parcel.weight * parcel.units,
        0,
      );
      const band = partial > 250 ? 'heavy' : 'light';
      return band === 'heavy' ? partial * 0.75 : partial;
    }),
  );
  return grands.filter((grand) => grand > 0);
}
