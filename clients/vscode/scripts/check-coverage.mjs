// [VSIX-TESTING-COVERAGE] [VSIX-WEBVIEW-COVERAGE] Enforce the VSIX coverage
// floors from the repo-root `coverage-thresholds.json`. Mirrors the Rust
// `_coverage_check` in the top-level Makefile — same 1% rounding slack, same
// ratchet discipline.
//
// Both halves of the extension are measured and both are gated here: the
// extension host (`extension-coverage.mjs`, instrumented at build time because
// the host writes no V8 profile — gh #440) and the webview bundle
// (`webview-coverage.mjs`, V8 in a real browser).
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  COVERAGE_GATES as GATES,
  enforceLineThreshold,
  loadThresholds,
  thresholdsPath,
  vsixRoot,
} from "./coverage-paths.mjs";

const flag = process.argv.slice(2).find((arg) => arg in GATES);
if (flag === undefined) {
  console.error(`usage: check-coverage.mjs ${Object.keys(GATES).join(" | ")}`);
  process.exit(1);
}
const gate = GATES[flag];

const vsixCfg = loadThresholds().vsix;
const target = Number(vsixCfg?.[gate.thresholdKey]);
if (!Number.isFinite(target)) {
  console.error(`${thresholdsPath} is missing .vsix.${gate.thresholdKey}`);
  process.exit(1);
}

const summaryPath = resolve(vsixRoot, "coverage", gate.coverageDir, "coverage-summary.json");
let summary;
try {
  summary = JSON.parse(readFileSync(summaryPath, "utf8"));
} catch (err) {
  console.error(`failed to read ${summaryPath}: ${(err && err.message) || err}`);
  process.exit(1);
}

const pct = Number(summary.total?.lines?.pct);
if (!Number.isFinite(pct)) {
  console.error(`${summaryPath} has no total.lines.pct`);
  process.exit(1);
}

process.exit(enforceLineThreshold(pct, target, gate.label));
