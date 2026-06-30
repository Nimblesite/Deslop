export { Worker } from "./worker";
export function summariseTotals(rows: number[]): number {
  let total = 0;
  for (const value of rows) {
    total = total + value;
  }
  return total;
}
