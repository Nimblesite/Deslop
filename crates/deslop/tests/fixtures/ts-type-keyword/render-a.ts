export function renderItems(items: ReadonlyArray<string>): string {
  let output: string = "";
  let total: number = 0;
  for (const item of items) {
    output = output + item;
    total = total + 1;
  }
  return output + total;
}
