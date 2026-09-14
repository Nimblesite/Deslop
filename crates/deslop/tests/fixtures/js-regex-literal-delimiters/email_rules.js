const emailPattern = /[a-z]+@[a-z]+/i;

export function isMailbox(candidate) {
  return emailPattern.test(candidate);
}
