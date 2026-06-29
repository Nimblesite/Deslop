type Widget = { id: string };

export function build(seed: string) {
  const list: Array<Widget> = [];
  const node: Widget = { id: seed };
  list.push(node);
  list.push(node);
  return list;
}
