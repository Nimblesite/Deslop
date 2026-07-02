export function parseHeaders(raw) {
  const headers = {};
  const pairs = raw.split("&");
  for (const pair of pairs) {
    const index = pair.indexOf("=");
    const key = decodeURIComponent(pair.slice(0, index));
    const value = decodeURIComponent(pair.slice(index + 1));
    headers[key.toLowerCase()] = value;
    headers[key] = value;
  }
  return headers;
}
