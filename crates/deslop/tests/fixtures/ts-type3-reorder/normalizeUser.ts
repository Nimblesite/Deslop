export function normalizeUser(input: Record<string, string>): Record<string, string> {
  const result: Record<string, string> = {};
  result.name = input.name.trim().toLowerCase();
  result.email = input.email.trim().toLowerCase();
  result.handle = input.handle.replace(/\s+/g, "");
  return result;
}
