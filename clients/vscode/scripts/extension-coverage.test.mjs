// [VSIX-TESTING-COVERAGE-RESTORE] The extension-host coverage run must fail closed.
//
// The run instruments `out/**` in place and recompiles it clean in a
// `finally`. Discarding that recompile's status — which the script did —
// lets the command print a coverage report and exit 0 with instrumented
// modules still staged: `vsix-package` would then ship them and every
// non-coverage suite would run against them, so a green gate would stand
// over code nobody measured. These assertions make that unrepresentable.
//
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { coverageRunExit } from "./coverage-paths.mjs";

/// Where the instrumented modules would be left behind.
const STAGED_PATH = "coverage/extension";
/// A non-zero status from the clean recompile.
const RESTORE_FAILED = 2;
/// The recompile succeeded.
const RESTORE_OK = 0;
/// How collection reported its own failure.
const COLLECT_FAILURE = "the extension suite failed (1)";
/// Exit codes, named so an assertion says which outcome it means.
const PASS = 0;
const FAIL = 1;

test("a clean run with a clean restore passes", () => {
  const outcome = coverageRunExit({ restore: RESTORE_OK, stagedPath: STAGED_PATH });
  assert.equal(outcome.code, PASS, "nothing failed, so the gate must not fail");
  assert.equal(outcome.reason, "", "a passing run has nothing to report");
});

test("a failed restore alone still fails the run", () => {
  const outcome = coverageRunExit({ restore: RESTORE_FAILED, stagedPath: STAGED_PATH });
  assert.equal(
    outcome.code,
    FAIL,
    "instrumented output left staged must never exit 0 — vsix-package would ship it",
  );
  assert.match(outcome.reason, /still staged/, "the reason must say what was left behind");
  assert.ok(
    outcome.reason.includes(STAGED_PATH),
    `the reason must name the staged path: ${outcome.reason}`,
  );
  assert.ok(
    outcome.reason.includes(String(RESTORE_FAILED)),
    `the reason must carry the recompile's status: ${outcome.reason}`,
  );
});

test("a failed collection fails the run and is reported on its own", () => {
  const outcome = coverageRunExit({
    failure: COLLECT_FAILURE,
    restore: RESTORE_OK,
    stagedPath: STAGED_PATH,
  });
  assert.equal(outcome.code, FAIL);
  assert.ok(
    outcome.reason.includes(COLLECT_FAILURE),
    `the reason must be the collection failure: ${outcome.reason}`,
  );
  assert.doesNotMatch(
    outcome.reason,
    /still staged/,
    "a successful restore must not be reported as staged output",
  );
});

test("a failed collection keeps a failed restore as context", () => {
  const outcome = coverageRunExit({
    failure: COLLECT_FAILURE,
    restore: RESTORE_FAILED,
    stagedPath: STAGED_PATH,
  });
  assert.equal(outcome.code, FAIL);
  assert.ok(
    outcome.reason.includes(COLLECT_FAILURE),
    `the failure that explains the run comes first: ${outcome.reason}`,
  );
  assert.match(
    outcome.reason,
    /still staged/,
    "the restore failure must survive alongside it, not be dropped",
  );
});

test("an empty failure string is not a failure", () => {
  // `failure?.message` is undefined on the clean path and could be "" if a
  // thrown error carried no message; neither may be read as a pass-through
  // that hides a staged tree.
  const outcome = coverageRunExit({ failure: "", restore: RESTORE_OK, stagedPath: STAGED_PATH });
  assert.equal(outcome.code, PASS);
});
