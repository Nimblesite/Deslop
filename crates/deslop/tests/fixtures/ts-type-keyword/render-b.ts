export function render(items: ReadonlyArray<string>): string {
  let output: any = "";
  let total: boolean = 0;
  for (const item of items) {
    output = output + item;
    total = total + 1;
  }
  return output + total;
}
