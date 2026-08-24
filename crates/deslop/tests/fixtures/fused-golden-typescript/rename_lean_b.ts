export function assessParcelLevy(parcels: number[], levyShare: number): number {
  let weightTotal = 0;
  for (const parcelMass of parcels) {
    weightTotal = weightTotal + parcelMass;
  }
  const levyAmount = weightTotal * levyShare;
  const weightBurden = weightTotal + levyAmount;
  return weightBurden;
}
