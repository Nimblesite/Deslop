// Shared `$GITHUB_OUTPUT` writer for the composite action's steps.
// [ACTION-RESOLVE], [ACTION-GATE].
//
// Every value the action publishes is single-line by construction (versions,
// counts, percentages, report paths), so the plain `key=value` form is used
// rather than a heredoc delimiter.

import { appendFileSync } from "node:fs";

/**
 * Appends each entry of `entries` to the step-output file named by
 * `$GITHUB_OUTPUT`.
 *
 * @param {Record<string, string>} entries
 * @returns {void}
 */
export function writeOutputs(entries) {
  const target = process.env.GITHUB_OUTPUT;
  if (!target) {
    throw new Error("GITHUB_OUTPUT is unset; this script runs inside a GitHub Actions step");
  }
  const lines = Object.entries(entries).map(([key, value]) => `${key}=${value}`);
  appendFileSync(target, `${lines.join("\n")}\n`);
}
