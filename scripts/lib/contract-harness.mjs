// The one assertion harness every contract suite shares. [ACTION-TESTS]
//
// Two idioms, one counter. The action suite spreads `check` calls across
// modules that run at import time, so the entry point asserts a floor on
// `total()` and a module that silently stops importing cannot pass. The
// release and deployment suites hand `runContractSuite` an array of named
// test functions instead, and want every test attempted before the exit code
// is decided — a red suite should name all of its failures, not just the
// first. A second copy of either loop would let the suites drift on the very
// reporting that decides whether a gate is under test at all.

import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

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
 * Runs every test in `tests`, printing TAP-style lines, then reports
 * `<n> <suiteLabel> tests passed` or exits 1 naming the failure count. A
 * `workdirPrefix` gives each test a fresh temp directory — passed as the
 * test's argument — removed win or lose.
 *
 * @param {Array<(work?: string) => void>} tests named test functions
 * @param {string} suiteLabel human-readable suite name for the summary line
 * @param {string} [workdirPrefix] prefix for a per-test temp directory
 */
export function runContractSuite(tests, suiteLabel, workdirPrefix) {
  const failed = tests.filter((test) => !runCase(test, workdirPrefix)).length;
  if (failed > 0) {
    console.error(`\n${failed} ${suiteLabel} test(s) failed`);
    process.exit(1);
  }
  console.log(`\n${tests.length} ${suiteLabel} tests passed`);
}

/** Runs one test in its own optional temp directory. @returns {boolean} */
function runCase(test, workdirPrefix) {
  const work = workdirPrefix === undefined ? undefined : mkdtempSync(join(tmpdir(), workdirPrefix));
  try {
    test(work);
    checked += 1;
    console.log(`ok ${test.name}`);
    return true;
  } catch (error) {
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
    return false;
  } finally {
    if (work !== undefined) rmSync(work, { recursive: true, force: true });
  }
}

/**
 * Number of checks run so far across the whole suite.
 *
 * @returns {number}
 */
export function total() {
  return checked;
}
