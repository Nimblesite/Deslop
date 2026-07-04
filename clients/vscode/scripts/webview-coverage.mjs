// [VSIX-WEBVIEW-COVERAGE] Webview code-coverage gate. The webview bundle (webview-ui/src ->
// media/webview/*.js) is invisible to the vscode-test c8 pass, which only sees
// the extension host under out/**. #254 (a value erased by `import type`) shipped
// straight through that blind spot. This closes it: build the bundle with inline
// sourcemaps, run the Playwright smoke suite with V8 coverage on, map the
// executed ranges back to webview-ui/src, and enforce the floor from the
// repo-root coverage-thresholds.json (.vsix.webview_threshold). Same ratchet +
// 1% rounding slack as check-coverage.mjs and the Rust _coverage_check.

import { readFileSync, readdirSync, writeFileSync, mkdirSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, relative, sep } from "node:path";
import v8toIstanbul from "v8-to-istanbul";
import libCoverage from "istanbul-lib-coverage";
import { enforceLineThreshold, loadThresholds, runTool, vsixRoot } from "./coverage-paths.mjs";

const webviewSrc = resolve(vsixRoot, "webview-ui", "src");
const rawDir = resolve(vsixRoot, "coverage", "webview", "raw");
const outDir = resolve(vsixRoot, "coverage", "webview");

const thresholds = loadThresholds();
const target = Number(thresholds.vsix?.webview_threshold);
if (!Number.isFinite(target)) {
  console.error("coverage-thresholds.json is missing .vsix.webview_threshold");
  process.exit(1);
}

// Build the instrumentable (unminified, inline-sourcemap) bundle, drive the
// smoke suite with V8 coverage on, and map the executed ranges back to source.
// Any failure in here must still restore the production bundle (finally), or a
// coverage build would be left staged in media/webview for packaging/tests.
function collect() {
  rmSync(rawDir, { recursive: true, force: true });
  let status = runTool("npm", ["--prefix", "webview-ui", "run", "build", "--", "--coverage"]);
  if (status !== 0) process.exit(status);
  status = runTool("npx", ["playwright", "test", "scripts/playwright-webview-smoke.spec.ts"], {
    WEBVIEW_COVERAGE: "1",
  });
  if (status !== 0) process.exit(status);
  return mapCoverage();
}

const isWebviewSource = (file) => {
  const rel = relative(webviewSrc, file);
  return rel.length > 0 && !rel.startsWith("..") && !rel.includes(`node_modules${sep}`);
};

// Relative sourcemap sources (e.g. ../../webview-ui/src/cluster/main.tsx) are
// resolved against the bundle's own directory, so anchor the converter there.
async function mapCoverage() {
  const map = libCoverage.createCoverageMap({});
  let rawFiles = [];
  try {
    rawFiles = readdirSync(rawDir).filter((name) => name.endsWith(".json"));
  } catch {
    console.error(`no raw coverage written to ${rawDir} — did the Playwright run produce any?`);
    process.exit(1);
  }
  const bundleBase = resolve(vsixRoot, "media", "webview", "bundle.js");
  for (const name of rawFiles) {
    const entries = JSON.parse(readFileSync(resolve(rawDir, name), "utf8"));
    for (const entry of entries) {
      const converter = v8toIstanbul(bundleBase, 0, { source: entry.source });
      await converter.load();
      converter.applyCoverage(entry.functions);
      const data = converter.toIstanbul();
      for (const [file, fileCov] of Object.entries(data)) {
        const abs = file.startsWith("file://") ? fileURLToPath(file) : file;
        if (isWebviewSource(abs)) map.merge({ [abs]: { ...fileCov, path: abs } });
      }
    }
  }
  return map;
}

let map;
try {
  map = await collect();
} finally {
  // A coverage run must never leave the unminified inline-sourcemap output
  // staged for packaging — restore the production bundle on every path.
  runTool("npm", ["--prefix", "webview-ui", "run", "build"]);
}

// Summarise per file + total, write coverage-summary.json, enforce the floor.
const totalSummary = libCoverage.createCoverageSummary();
const perFile = [];
for (const file of map.files()) {
  const fileSummary = map.fileCoverageFor(file).toSummary();
  totalSummary.merge(fileSummary);
  perFile.push([relative(webviewSrc, file), fileSummary.lines.pct]);
}
mkdirSync(outDir, { recursive: true });
writeFileSync(
  resolve(outDir, "coverage-summary.json"),
  JSON.stringify({ total: totalSummary.toJSON() }, null, 2),
);

perFile.sort((a, b) => a[1] - b[1]);
console.log("\nWebview line coverage by file:");
for (const [file, pct] of perFile) console.log(`  ${pct.toFixed(1).padStart(6)}%  ${file}`);

const pct = Number(totalSummary.toJSON().lines.pct);
if (perFile.length === 0 || !Number.isFinite(pct)) {
  console.error("FAIL: no webview-ui/src coverage was mapped — the harness is broken, not passing by default");
  process.exit(1);
}
console.log("");
process.exit(enforceLineThreshold(pct, target, "Webview"));
