export function projectVertex(source) {
  const { x, y, z = 0, ...remainder } = source;
  const [head, tail, ...extra] = source.path;
  const doubled = { x: x * 2, y: y * 2, z: z * 2 };
  return { doubled, head, tail, extra, remainder };
}
