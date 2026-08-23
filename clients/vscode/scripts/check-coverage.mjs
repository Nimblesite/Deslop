// [VSIX-WEBVIEW-COVERAGE] Enforce the webview coverage floor from the
// repo-root `coverage-thresholds.json`. Mirrors the Rust `_coverage_check`
// in the top-level Makefile — same 1% rounding slack, same ratchet
// discipline. Reads the summary `webview-coverage.mjs` writes.
//
// The extension host's own out/** code has no measurable coverage channel:
// the desktop extension host ignores NODE_V8_COVERAGE on every injection
// path for plain-Mocha suites, so no floor for it exists — gh #440.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { enforceLineThreshold, loadThresholds, thresholdsPath, vsixRoot } from "./coverage-paths.mjs";

const thresholds = loadThresholds();
const vsixCfg = thresholds.vsix;
if (!vsixCfg || !Number.isFinite(Number(vsixCfg.webview_threshold))) {
  console.error(`${thresholdsPath} is missing .vsix.webview_threshold`);
  process.exit(1);
}
const target = Number(vsixCfg.webview_threshold);

const summaryPath = resolve(vsixRoot, "coverage", "webview", "coverage-summary.json");
let summary;
try {
  summary = JSON.parse(readFileSync(summaryPath, "utf8"));
} catch (err) {
  console.error(`failed to read ${summaryPath}: ${(err && err.message) || err}`);
  process.exit(1);
}

const pct = Number(summary.total?.lines?.pct);
if (!Number.isFinite(pct)) {
  console.error("webview coverage-summary.json has no total.lines.pct");
  process.exit(1);
}

process.exit(enforceLineThreshold(pct, target, "Webview"));
