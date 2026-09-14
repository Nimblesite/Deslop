// Shared assertion harness for the action contract suite. [ACTION-TESTS]
//
// One counter across every module of the suite, so the entry point can print a
// single total and a silent regression to zero checks stays impossible.

import assert from "node:assert/strict";

let checked = 0;

/**
 * Runs one labelled check and counts it. A throw inside `body` fails the whole
 * suite immediately — the runner is `node`, and an uncaught error is exit 1.
 *
 * @param {string} label human-readable description of the property checked
 * @param {() => void} body assertions proving the property
 */
export function check(label, body) {
  body();
  checked += 1;
  console.log(`  ok  ${label}`);
}

/**
 * Asserts `body` throws an error whose message mentions `needle`.
 *
 * @param {string} label human-readable description of the property checked
 * @param {() => void} body call expected to throw
 * @param {string} needle substring the error message must carry
 */
export function expectThrows(label, body, needle) {
  check(label, () => {
    assert.throws(body, (error) => {
      assert.ok(
        error.message.includes(needle),
        `expected the error to mention "${needle}", got "${error.message}"`,
      );
      return true;
    });
  });
}

/**
 * Number of checks run so far across the whole suite.
 *
 * @returns {number}
 */
export function total() {
  return checked;
}
