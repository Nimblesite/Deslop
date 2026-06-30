// Shared path roots + threshold loader for the two VSIX coverage gates
// (check-coverage.mjs = extension host, webview-coverage.mjs = webview bundle).
// One resolver keeps the repo-root coverage-thresholds.json the single source of
// truth and stops the path math from being copy-pasted per gate.
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
