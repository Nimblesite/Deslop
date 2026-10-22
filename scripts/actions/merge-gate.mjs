// The merge gate behind the required "CI" status. [CI-MERGE-GATE]
//
// The branch ruleset requires a status named "CI", and GitHub counts a
// SKIPPED required check as passing. So this gate must always run and must
// never treat a skip as a pass unless the change genuinely did not need that
// job. A job that failed, was cancelled, or was skipped when the change needed
// it fails the gate, and so blocks the merge.
//
// Usage (from the workflow): NEEDS='${{ toJSON(needs) }}'
// CODE='${{ needs.changes.outputs.code }}' SITE='${{ needs.changes.outputs.site }}'
// node scripts/actions/merge-gate.mjs

import { pathToFileURL } from "node:url";

/** The conclusion every job must reach to pass. */
export const SUCCESS = "success";
/** The conclusion a job reaches when its `if:` excluded it. */
export const SKIPPED = "skipped";
/** Jobs every change needs, whatever it touched. */
export const ALWAYS_REQUIRED = ["changes", "security"];
/** The job a website change needs even when no code changed. */
export const SITE_JOB = "site";
/** What the path classifier writes for a change that needs a suite. */
const CHANGED = "true";

/** Whether this change needed `job` to run. */
function required(job, scope) {
  return (
    ALWAYS_REQUIRED.includes(job) ||
    scope.code === CHANGED ||
    (job === SITE_JOB && scope.site === CHANGED)
  );
}

/** Every reason the gate fails, one per offending job; empty when it passes. */
export function gateFailures(needs, scope) {
  const missing = ALWAYS_REQUIRED.filter((job) => !(job in needs)).map(
    (job) => `${job} is not a dependency of the gate`,
  );
  const offending = Object.entries(needs).flatMap(([job, { result }]) => {
    if (result === SUCCESS) return [];
    if (result === SKIPPED && !required(job, scope)) return [];
    if (result === SKIPPED) return [`${job} was skipped, but this change needs it`];
    return [`${job} ended '${result}'`];
  });
  return [...missing, ...offending];
}

/** Reads the workflow's environment and exits non-zero on any failure. */
function main() {
  const needs = JSON.parse(process.env.NEEDS ?? "{}");
  const failures = gateFailures(needs, { code: process.env.CODE, site: process.env.SITE });
  for (const [job, { result }] of Object.entries(needs)) console.log(`${job}: ${result}`);
  for (const failure of failures) console.error(`FAIL: ${failure}`);
  process.exitCode = failures.length === 0 ? 0 : 1;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main();
