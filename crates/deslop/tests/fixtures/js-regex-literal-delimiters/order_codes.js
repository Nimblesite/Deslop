const orderPattern = /[0-9]{3}-[0-9]{4}/g;

export function countOrderCodes(ledger) {
  return ledger.match(orderPattern).length;
}
