export function parseQuery(raw) {
  const params = {};
  const pairs = raw.split("&");
  for (const pair of pairs) {
    const index = pair.indexOf("=");
    const key = decodeURIComponent(pair.slice(0, index));
    const value = decodeURIComponent(pair.slice(index + 1));
    params[key] = value;
  }
  return params;
}
