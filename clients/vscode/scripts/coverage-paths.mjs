// Shared plumbing for the VSIX coverage gates (check-coverage.mjs = extension
// host, webview-coverage.mjs = webview bundle): path roots, the repo-root
// coverage-thresholds.json loader, the spawn wrapper, and the threshold
// enforcement — one copy each, per the aggressively-DRY rule.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
/** clients/vscode — the VSIX project root. */
export const vsixRoot = resolve(here, "..");
/** Repository root. */
export const repoRoot = resolve(vsixRoot, "..", "..");
/** The single source of truth for every coverage floor. */
export const thresholdsPath = resolve(repoRoot, "coverage-thresholds.json");

/**
 * The VSIX coverage gates, keyed by CLI flag. One entry per measured surface.
 * Every `.vsix` floor in coverage-thresholds.json must be claimed by a gate
 * here and every gate must name a floor that exists — an unclaimed floor is a
 * number nothing enforces, which is how `projects{}` sat in that file for so
 * long declaring a 95% VSIX threshold that no code ever read.
 */
export const COVERAGE_GATES = {
  "--extension": {
    thresholdKey: "extension_threshold",
    coverageDir: "extension",
    label: "Extension-host",
  },
  "--webview": {
    thresholdKey: "webview_threshold",
    coverageDir: "webview",
    label: "Webview",
  },
};

/** Parse the repo-root coverage-thresholds.json. */
export function loadThresholds() {
  return JSON.parse(readFileSync(thresholdsPath, "utf8"));
}

/**
 * Spawn a tool from the VSIX root with inherited stdio and return its exit
 * status. `shell` is required on win32 to resolve `.cmd` shims (same pattern
 * as package-vsix.mjs), and a spawn failure is surfaced, never swallowed.
 */
export function runTool(cmd, args, env) {
  const result = spawnSync(cmd, args, {
    cwd: vsixRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: { ...process.env, ...env },
  });
  if (result.error) {
    console.error(`failed to spawn ${cmd} ${args.join(" ")}: ${result.error.message}`);
  }
  return result.status ?? 1;
}

/**
 * Enforce a line-coverage floor with the shared 1% rounding slack and return
 * the process exit code. Mirrors the Rust `_coverage_check` discipline.
 */
export function enforceLineThreshold(pct, target, label) {
  console.log(`${label} line coverage: ${pct.toFixed(1)}% (threshold: ${target}% + 1% slack)`);
  if (pct + 1.0 < target) {
    console.error(`FAIL: ${pct.toFixed(1)}% + 1% slack < ${target}%`);
    return 1;
  }
  console.log(`OK: ${pct.toFixed(1)}% + 1% slack >= ${target}%`);
  return 0;
}

/**
 * [VSIX-TESTING-COVERAGE-RESTORE] The exit code for one extension-host coverage run,
 * from how collection ended and how the clean recompile that restores
 * `out/**` ended.
 *
 * Both are terminal. A restore failure on its own must still fail the
 * command: exiting 0 there leaves instrumented modules staged in `out/**`
 * for `vsix-package` to ship and for every non-coverage suite to run
 * against — a green gate over code nobody measured. When collection failed
 * too, that failure is reported first because it explains the run, and the
 * restore failure is kept as context rather than dropped.
 *
 * @param {{failure?: string, restore: number, stagedPath: string}} run
 * @returns {{code: number, reason: string}}
 */
export function coverageRunExit({ failure, restore, stagedPath }) {
  const staged = `the clean recompile failed (${restore}); instrumented ${stagedPath} is still staged`;
  if (failure !== undefined && failure !== "") {
    return {
      code: 1,
      reason: restore === 0 ? `FAIL: ${failure}` : `FAIL: ${failure}; also ${staged}`,
    };
  }
  return restore === 0 ? { code: 0, reason: "" } : { code: 1, reason: `FAIL: ${staged}` };
}
