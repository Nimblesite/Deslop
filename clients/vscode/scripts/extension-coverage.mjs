// [VSIX-TESTING-COVERAGE] Extension-host code-coverage gate.
//
// The extension host emits no V8 profile for our code: a raw
// `NODE_V8_COVERAGE` run captures 1171 scripts and not one of them belongs to
// this extension — that profile is VS Code's main process, and the host that
// actually loads the extension never writes one (gh #440). Pointing c8 at a
// different artifact cannot recover a profile that was never taken, so the
// counters are compiled into the code instead and dumped by a hook running
// inside the host.
//
// `out/**` is the artifact measured, not `dist/extension.js`. The extension
// ships as the bundle, but the suites import modules directly
// (`../../decorations/...`), which resolves to the separate `out/` copy — so
// the bundle sees almost none of the test suite's work and instrumenting it
// scored every unit-tested module 0% (57.98% overall against 87.56% here).
//
// The two copies cannot be merged: they are compiled by different toolchains,
// so their statement maps disagree and istanbul would combine tables that do
// not describe the same code. Measuring `out/**` alone therefore UNDER-reports
// by whatever the E2E activation path exercises only through the bundle, which
// is the safe direction for a floor — it can never claim coverage that no test
// produced.
//
// The denominator is the instrumentation baseline — every compiled module,
// whether or not a test loads it — so the percentage answers "how much of the
// extension is tested", not "how much of what happened to load is tested".

import {
  readFileSync,
  readdirSync,
  writeFileSync,
  mkdirSync,
  rmSync,
} from "node:fs";
import { resolve, relative } from "node:path";
import libCoverage from "istanbul-lib-coverage";
import { coverageRunExit, runTool, vsixRoot } from "./coverage-paths.mjs";

const outRoot = resolve(vsixRoot, "out");
const outDir = resolve(vsixRoot, "coverage", "extension");
const rawDir = resolve(outDir, "raw");
const baselinePath = resolve(outDir, "baseline-out.json");
/// Handed to the extension host so the dump hook knows where to write.
const COVERAGE_DIR_ENV = "DESLOP_EXTENSION_COVERAGE_DIR";

/// A step that failed. Thrown, never `process.exit`ed: an exit here would skip
/// the restore in `finally` and leave instrumented output staged.
class StepFailed extends Error {}

function collect() {
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(rawDir, { recursive: true });
  let status = runTool("npm", ["run", "compile"]);
  if (status !== 0) throw new StepFailed(`compile failed (${status})`);
  status = runTool("node", ["./scripts/instrument-out.mjs"]);
  if (status !== 0) throw new StepFailed(`instrumentation failed (${status})`);
  status = runTool("npx", ["vscode-test"], { [COVERAGE_DIR_ENV]: rawDir });
  if (status !== 0)
    throw new StepFailed(`the extension suite failed (${status})`);
}

function readBaseline() {
  try {
    return JSON.parse(readFileSync(baselinePath, "utf8"));
  } catch (err) {
    console.error(
      `FAIL: no instrumentation baseline at ${baselinePath}: ${err?.message ?? err}`,
    );
    process.exit(1);
  }
}

/// Merge the baseline with whatever the host executed.
function buildMap(baseline) {
  const map = libCoverage.createCoverageMap({});
  map.merge(baseline);
  let dumps = 0;
  for (const name of readdirSync(rawDir).filter((file) =>
    file.endsWith(".json"),
  )) {
    map.merge(JSON.parse(readFileSync(resolve(rawDir, name), "utf8")));
    dumps += 1;
  }
  if (dumps === 0) {
    console.error(`FAIL: the extension host wrote no coverage to ${rawDir}`);
    process.exit(1);
  }
  return map;
}

let failure;
let restore = 0;
try {
  collect();
} catch (err) {
  failure = err;
} finally {
  // [VSIX-TESTING-COVERAGE-RESTORE] Never leave instrumented output staged for packaging or for the
  // non-coverage suites — recompile clean on every path. The status is
  // kept, not discarded: a silent failure here exits 0 with instrumented
  // modules still in out/**, which `vsix-package` would then ship and
  // every non-coverage suite would then run against.
  restore = runTool("npm", ["run", "compile"]);
}
const outcome = coverageRunExit({
  failure: failure?.message,
  restore,
  // The instrumented artifact is `out/**`, which the restore compile
  // overwrites — not the coverage report directory. Naming the wrong path
  // here sends a reader to a directory that was never instrumented.
  stagedPath: outRoot,
});
if (outcome.code !== 0) {
  console.error(outcome.reason);
  process.exit(outcome.code);
}

const baseline = readBaseline();
const map = buildMap(baseline);

// The report must describe the whole extension. If the run dropped or added a
// module the denominator is not the extension and the number is not the truth,
// so refuse it rather than print it.
const expected = Object.keys(baseline).sort();
const actual = map.files().sort();
if (expected.length !== actual.length) {
  console.error(
    `FAIL: coverage covers ${actual.length} modules but instrumentation produced ${expected.length}`,
  );
  for (const file of expected.filter((name) => !actual.includes(name))) {
    console.error(`  missing: ${relative(outRoot, file)}`);
  }
  process.exit(1);
}

const totalSummary = libCoverage.createCoverageSummary();
const perFile = [];
for (const file of map.files()) {
  const fileSummary = map.fileCoverageFor(file).toSummary();
  totalSummary.merge(fileSummary);
  perFile.push([
    relative(outRoot, file).replace(/\.js$/, ".ts"),
    fileSummary.lines.pct,
  ]);
}

mkdirSync(outDir, { recursive: true });
writeFileSync(
  resolve(outDir, "coverage-summary.json"),
  JSON.stringify({ total: totalSummary.toJSON() }, null, 2),
);

perFile.sort((a, b) => a[1] - b[1]);
console.log(
  `\nExtension-host line coverage by file (${perFile.length} modules):`,
);
for (const [file, pct] of perFile)
  console.log(`  ${pct.toFixed(1).padStart(6)}%  ${file}`);

const pct = Number(totalSummary.toJSON().lines.pct);
if (perFile.length === 0 || !Number.isFinite(pct)) {
  console.error(
    "FAIL: no extension coverage was mapped — the harness is broken, not passing by default",
  );
  process.exit(1);
}
console.log("");
