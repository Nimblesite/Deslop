export function projectPoint(input) {
  const { x, y, z = 0, ...rest } = input;
  const [first, second, ...others] = input.path;
  const scaled = { x: x * 2, y: y * 2, z: z * 2 };
  return { scaled, first, second, others, rest };
}
