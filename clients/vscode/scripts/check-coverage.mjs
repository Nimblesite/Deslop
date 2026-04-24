// Enforce VSIX coverage threshold from the repo-root
// `coverage-thresholds.json`. Mirrors the Rust `_coverage_check` in the
// top-level Makefile — same 1% rounding slack, same ratchet discipline.
//
// Single source of truth: ../../coverage-thresholds.json → .vsix.default_threshold.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const vsixRoot = resolve(here, "..");
const repoRoot = resolve(vsixRoot, "..", "..");
const thresholdsPath = resolve(repoRoot, "coverage-thresholds.json");

const thresholds = JSON.parse(readFileSync(thresholdsPath, "utf8"));
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

const pass = pct + 1.0 >= target;
console.log(`VSIX line coverage: ${pct.toFixed(1)}% (threshold: ${target}% + 1% slack)`);
if (!pass) {
  console.error(`FAIL: ${pct.toFixed(1)}% + 1% slack < ${target}%`);
  process.exit(1);
}
console.log(`OK: ${pct.toFixed(1)}% + 1% slack >= ${target}%`);
