// Shared runner for the deployment contract suites ([DEPLOY-CI-GATES]).
// Every suite passes its test functions here: each test either returns
// silently or throws, the runner prints TAP-style ok / not ok lines, and a
// red suite exits 1 only after every test has run. A `workdirPrefix` gives
// each test a fresh temp directory (passed as the test's argument) that is
// removed win or lose.

import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/**
 * Runs `tests` and reports `<n> <suiteLabel> tests passed` or exits 1.
 */
export function runContractSuite(tests, suiteLabel, workdirPrefix) {
  const failed = tests.filter((test) => !runCase(test, workdirPrefix)).length;
  if (failed > 0) {
    console.error(`\n${failed} ${suiteLabel} test(s) failed`);
    process.exit(1);
  }
  console.log(`\n${tests.length} ${suiteLabel} tests passed`);
}

function runCase(test, workdirPrefix) {
  const work = workdirPrefix === undefined ? undefined : mkdtempSync(join(tmpdir(), workdirPrefix));
  try {
    test(work);
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
