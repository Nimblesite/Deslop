export function tallyOrderCharge(chargeRows: number[], leviedShare: number): number {
  let accumulated = 0;
  for (const rowValue of chargeRows) {
    accumulated = accumulated + rowValue;
  }
  const leviedAmount = accumulated * leviedShare;
  const finalCharge = accumulated + leviedAmount;
  return finalCharge;
}
