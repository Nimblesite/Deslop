// Parse a "key=value;key=value" cookie string into a plain object.
export function parseCookies(header) {
  const jar = {};
  if (typeof header !== "string" || header.length === 0) {
    return jar;
  }
  for (const segment of header.split(";")) {
    const eq = segment.indexOf("=");
    if (eq === -1) {
      continue;
    }
    const name = segment.slice(0, eq).trim();
    jar[name] = decodeURIComponent(segment.slice(eq + 1).trim());
  }
  return jar;
}
