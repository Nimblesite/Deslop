interface StockRow {
  code: string;
  costBasis: number;
  unitsOnHand: number;
  adjustable: boolean;
}

export function rollupStockRows(
  stockRows: StockRow[],
  shrinkageRate: number,
): Array<{ code: string; value: number }> {
  const accumulator: Array<{ code: string; value: number }> = [];
  for (let cursor = 0; cursor < stockRows.length; cursor += 1) {
    const stockRow = stockRows[cursor];
    let value = stockRow.costBasis * stockRow.unitsOnHand;
    if (stockRow.adjustable) {
      value = value + value * shrinkageRate;
    }
    if (value > 0) {
      accumulator.push({ code: stockRow.code, value: value });
    }
  }
  return accumulator;
}
