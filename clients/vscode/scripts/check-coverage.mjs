// [VSIX-TESTING-COVERAGE] Enforce the VSIX extension-host coverage floor from
// the repo-root `coverage-thresholds.json`. Mirrors the Rust `_coverage_check`
// in the top-level Makefile — same 1% rounding slack, same ratchet discipline.
// Reads the c8 summary that `vscode-test --coverage` writes over `out/**`.
//
// Single source of truth: ../../coverage-thresholds.json → .vsix.default_threshold.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { enforceLineThreshold, loadThresholds, thresholdsPath, vsixRoot } from "./coverage-paths.mjs";

const thresholds = loadThresholds();
const vsixCfg = thresholds.vsix;
if (!vsixCfg || !Number.isFinite(Number(vsixCfg.default_threshold))) {
  console.error(`${thresholdsPath} is missing .vsix.default_threshold`);
  process.exit(1);
}
const target = Number(vsixCfg.default_threshold);

const summaryPath = resolve(vsixRoot, "coverage", "coverage-summary.json");
let summary;
try {
  summary = JSON.parse(readFileSync(summaryPath, "utf8"));
} catch (err) {
  console.error(`failed to read ${summaryPath}: ${(err && err.message) || err}`);
  process.exit(1);
}

const pct = Number(summary.total?.lines?.pct);
if (!Number.isFinite(pct)) {
  console.error("coverage-summary.json has no total.lines.pct");
  process.exit(1);
}

process.exit(enforceLineThreshold(pct, target, "VSIX"));
