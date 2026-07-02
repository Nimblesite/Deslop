// Shared plumbing for the VSIX coverage gates (check-coverage.mjs = extension
// host, extension-coverage.mjs = entry swap, webview-coverage.mjs = webview
// bundle): path roots, the repo-root coverage-thresholds.json loader, the
// spawn wrapper, and the threshold enforcement — one copy each, per the
// aggressively-DRY rule.
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
