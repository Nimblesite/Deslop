export function validateHandle(input) {
  const matcher = /^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$/i;
  const trimmed = input.trim().replace(/\s+/g, "");
  if (!matcher.test(trimmed)) {
    return { ok: false, value: trimmed };
  }
  return { ok: true, value: trimmed };
}
