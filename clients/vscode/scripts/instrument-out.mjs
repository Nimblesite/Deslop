// [VSIX-TESTING-COVERAGE] Instruments the tsc output the unit suites load.
//
// The extension ships as `dist/extension.js`, but the unit suites import
// modules directly (`../../decorations/clusterHoverProvider`), which resolves
// to a SECOND compiled copy under `out/`. Instrumenting only the bundle
// therefore scores every unit-tested module at 0% — under-reporting by the
// exact amount the unit suites cover.
//
// Both copies are instrumented against their original `src/**/*.ts` path, so
// executing either credits the same source file and the two merge into one
// honest number.

import { readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { resolve, join } from "node:path";
import { createInstrumenter } from "istanbul-lib-instrument";
import { vsixRoot } from "./coverage-paths.mjs";

const outRoot = resolve(vsixRoot, "out");
const testDir = resolve(outRoot, "test");
const baselinePath = resolve(vsixRoot, "coverage", "extension", "baseline-out.json");

/// Every compiled module the suites can load, minus the suites themselves.
function* compiledModules(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      if (full === testDir) continue;
      yield* compiledModules(full);
    } else if (full.endsWith(".js")) {
      yield full;
    }
  }
}

const instrumenter = createInstrumenter({
  esModules: false,
  produceSourceMap: true,
  coverageVariable: "__coverage__",
  compact: false,
});

const baseline = {};
let count = 0;
for (const file of compiledModules(outRoot)) {
  const code = readFileSync(file, "utf8");
  if (code.includes("__coverage__")) continue; // already instrumented
  let inputSourceMap;
  try {
    inputSourceMap = JSON.parse(readFileSync(`${file}.map`, "utf8"));
  } catch {
    inputSourceMap = undefined;
  }
  const instrumented = instrumenter.instrumentSync(code, file, inputSourceMap);
  writeFileSync(file, instrumented);
  const fileCoverage = instrumenter.lastFileCoverage();
  baseline[fileCoverage.path] = fileCoverage;
  count += 1;
}

if (count === 0) {
  console.error("FAIL: instrumented no compiled modules under out/ — was `npm run compile` run?");
  process.exit(1);
}

mkdirSync(resolve(vsixRoot, "coverage", "extension"), { recursive: true });
writeFileSync(baselinePath, JSON.stringify(baseline));
console.log(`instrumented ${count} compiled modules -> ${baselinePath}`);
