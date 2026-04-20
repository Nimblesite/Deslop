// Enforce the VSIX coverage threshold from coverage-thresholds.json.
// Mirrors the Rust _coverage_check in the repo Makefile.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const thresholds = JSON.parse(
  readFileSync(resolve(root, "coverage-thresholds.json"), "utf8"),
);
const target = Number(thresholds.default_threshold);

const summaryPath = resolve(root, "coverage", "coverage-summary.json");
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

const pass = pct + 1.0 >= target; // same 1% rounding slack as the Rust check
console.log(`VSIX line coverage: ${pct.toFixed(1)}% (threshold: ${target}% + 1% slack)`);
if (!pass) {
  console.error(`FAIL: ${pct.toFixed(1)}% + 1% slack < ${target}%`);
  process.exit(1);
}
console.log(`OK: ${pct.toFixed(1)}% + 1% slack >= ${target}%`);
