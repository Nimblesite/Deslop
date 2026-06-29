export function randomToken(length) {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  let token = "";
  while (token.length < length) {
    const pick = Math.floor(Math.random() * alphabet.length);
    token += alphabet.charAt(pick);
  }
  return token;
}
