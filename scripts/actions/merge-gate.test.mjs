// [CI-MERGE-GATE] The required "CI" status passes only when every job the
// change needs succeeded. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { gateFailures, SKIPPED, SUCCESS } from "./merge-gate.mjs";

const GATE = resolve(repoRoot, "scripts/actions/merge-gate.mjs");
const FAILURE = "failure";
const CANCELLED = "cancelled";
const CHANGED = "true";
const UNCHANGED = "false";
/** A code change: every job runs. */
const CODE_CHANGE = { code: CHANGED, site: CHANGED };
/** A website-only change: the code suites are skipped by design. */
const SITE_CHANGE = { code: UNCHANGED, site: CHANGED };
/** What `changes` leaves behind when it fails: no outputs at all. */
const NO_SCOPE = { code: "", site: "" };
const CODE_JOBS = ["lint", "build", "rust-tests", "repository-tests", "corpus-score", "coverage"];

/** The gate's `needs` with every job at `result`, then `overrides` applied. */
function needs(result, overrides = {}) {
  const jobs = ["changes", "security", "site", ...CODE_JOBS];
  const all = Object.fromEntries(jobs.map((job) => [job, { result }]));
  for (const [job, value] of Object.entries(overrides)) all[job] = { result: value };
  return all;
}

/** Runs the gate script the way the workflow does. */
function runGate(jobs, scope) {
  return spawnSync(process.execPath, [GATE], {
    env: { ...process.env, NEEDS: JSON.stringify(jobs), CODE: scope.code, SITE: scope.site },
    encoding: "utf8",
  });
}

test("[CI-MERGE-GATE] a code change passes only when every job succeeded", () => {
  assert.deepEqual(gateFailures(needs(SUCCESS), CODE_CHANGE), []);
  assert.deepEqual(gateFailures(needs(SUCCESS, { lint: FAILURE }), CODE_CHANGE), ["lint ended 'failure'"]);
  assert.deepEqual(gateFailures(needs(SUCCESS, { vsix: CANCELLED }), CODE_CHANGE), ["vsix ended 'cancelled'"]);
  assert.deepEqual(
    gateFailures(needs(SUCCESS, { lint: FAILURE, build: SKIPPED }), CODE_CHANGE),
    ["lint ended 'failure'", "build was skipped, but this change needs it"],
  );
});

test("[CI-MERGE-GATE] a failed or skipped classifier blocks the merge", () => {
  const skippedEverything = needs(SKIPPED, { changes: FAILURE, security: SUCCESS });
  assert.deepEqual(gateFailures(skippedEverything, NO_SCOPE), ["changes ended 'failure'"]);
  assert.deepEqual(gateFailures(needs(SKIPPED, { security: SUCCESS }), NO_SCOPE), [
    "changes was skipped, but this change needs it",
  ]);
  assert.deepEqual(gateFailures(needs(SUCCESS, { security: SKIPPED }), CODE_CHANGE), [
    "security was skipped, but this change needs it",
  ]);
});

test("[CI-MERGE-GATE] a website-only change may skip the code suites but never the site build", () => {
  const siteOnly = needs(SKIPPED, { changes: SUCCESS, security: SUCCESS, site: SUCCESS });
  assert.deepEqual(gateFailures(siteOnly, SITE_CHANGE), []);
  assert.deepEqual(gateFailures({ ...siteOnly, site: { result: FAILURE } }, SITE_CHANGE), ["site ended 'failure'"]);
  assert.deepEqual(gateFailures({ ...siteOnly, site: { result: SKIPPED } }, SITE_CHANGE), [
    "site was skipped, but this change needs it",
  ]);
  assert.deepEqual(gateFailures({ ...siteOnly, lint: { result: FAILURE } }, SITE_CHANGE), ["lint ended 'failure'"]);
});

test("[CI-MERGE-GATE] a gate that lost its classifier or security dependency fails", () => {
  const { changes: _changes, security: _security, ...rest } = needs(SUCCESS);
  assert.deepEqual(gateFailures(rest, CODE_CHANGE), [
    "changes is not a dependency of the gate",
    "security is not a dependency of the gate",
  ]);
});

test("[CI-MERGE-GATE] the script exits non-zero and names the job, or exits zero on a clean run", () => {
  const failed = runGate(needs(SUCCESS, { "rust-tests": FAILURE }), CODE_CHANGE);
  assert.equal(failed.status, 1, failed.stderr);
  assert.ok(failed.stderr.includes("FAIL: rust-tests ended 'failure'"), failed.stderr);
  const passed = runGate(needs(SUCCESS), CODE_CHANGE);
  assert.equal(passed.status, 0, passed.stderr);
  assert.equal(passed.stderr, "");
});
