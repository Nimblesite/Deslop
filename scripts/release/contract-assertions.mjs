// Membership assertions shared by the release contract suites.
//
// `test-release-workflow-contract.mjs` asserts over workflow *text* and
// `test-dependabot-gate-matrix.mjs` asserts over parsed *lists* of branch names.
// Both are `.includes()` against a haystack, so a copy in each file is the kind
// of near-duplicate this repository's own tool exists to find — it lives here
// instead, and the two suites import it.

/**
 * Assert `needle` is present in `haystack` (a string or an array).
 *
 * @param {string | readonly unknown[]} haystack value searched
 * @param {unknown} needle value required to be present
 * @param {string} message failure description
 */
export function assertIncludes(haystack, needle, message) {
  if (!haystack.includes(needle)) throw new Error(withFound(message, haystack));
}

/**
 * Assert `needle` is absent from `haystack` (a string or an array).
 *
 * @param {string | readonly unknown[]} haystack value searched
 * @param {unknown} needle value required to be absent
 * @param {string} message failure description
 */
export function assertExcludes(haystack, needle, message) {
  if (haystack.includes(needle)) throw new Error(withFound(message, haystack));
}

// A parsed branch list is short enough to name in the failure, and naming it is
// what makes the message actionable. Workflow text is not: dumping a multi-KB
// file into the message buries the assertion that actually failed.
function withFound(message, haystack) {
  return Array.isArray(haystack) ? `${message} (found: ${JSON.stringify(haystack)})` : message;
}
