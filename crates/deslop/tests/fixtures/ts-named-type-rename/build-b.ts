type Gadget = { id: string };

export function build(seed: string) {
  const list: Array<Gadget> = [];
  const node: Gadget = { id: seed };
  list.push(node);
  list.push(node);
  return list;
}
