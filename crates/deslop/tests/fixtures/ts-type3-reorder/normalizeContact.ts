export function normalizeContact(input: Record<string, string>): Record<string, string> {
  const result: Record<string, string> = {};
  result.email = input.email.trim().toLowerCase();
  result.name = input.name.trim().toLowerCase();
  result.handle = input.handle.replace(/\s+/g, "");
  return result;
}
