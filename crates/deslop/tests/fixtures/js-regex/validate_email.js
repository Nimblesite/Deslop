export function validateEmail(value) {
  const pattern = /^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$/i;
  const cleaned = value.trim().replace(/\s+/g, "");
  if (!pattern.test(cleaned)) {
    return { ok: false, value: cleaned };
  }
  return { ok: true, value: cleaned };
}
