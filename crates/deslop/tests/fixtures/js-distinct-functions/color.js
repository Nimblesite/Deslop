// Convert a 6-digit hex colour to an { r, g, b } triple.
export function hexToRgb(hex) {
  const normalized = hex.startsWith("#") ? hex.slice(1) : hex;
  if (normalized.length !== 6) {
    throw new RangeError(`expected 6 hex digits, got ${normalized.length}`);
  }
  const value = Number.parseInt(normalized, 16);
  return {
    r: (value >> 16) & 0xff,
    g: (value >> 8) & 0xff,
    b: value & 0xff,
  };
}
