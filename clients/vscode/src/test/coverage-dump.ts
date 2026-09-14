// [VSIX-TESTING-COVERAGE] Writes the extension host's coverage table to disk.
//
// The counters live inside the bundle (see scripts/istanbul-esbuild-plugin.mjs),
// so they accumulate on the extension host's own global as the suites drive the
// extension. Mocha runs in that same process, which makes a root hook the one
// place that can see them before the host exits.
//
// Loaded via the `mocha.require` entry in .vscode-test.mjs. That entry is
// plain-`require`d by the test-cli runner, which then deletes it and never
// registers an exported `mochaHooks` — so the write is bound to the host's own
// `exit` event rather than to a Mocha root hook. Inert unless
// DESLOP_EXTENSION_COVERAGE_DIR is set, so `npm test` is unaffected.

import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

/// Set by scripts/extension-coverage.mjs to switch collection on.
const COVERAGE_DIR_ENV = "DESLOP_EXTENSION_COVERAGE_DIR";
/// The global the instrumenter accumulates into.
const COVERAGE_VARIABLE = "__coverage__";

/** The istanbul coverage table keyed by absolute source path. */
type CoverageTable = Record<string, unknown>;

function dump(dir: string): void {
  const table = (globalThis as Record<string, unknown>)[COVERAGE_VARIABLE] as
    | CoverageTable
    | undefined;
  if (table === undefined || Object.keys(table).length === 0) {
    // Collection was requested and produced nothing. Writing an empty table
    // here would merge to a clean 0% and read as a real measurement, so leave
    // the directory bare and let the collector refuse the run instead.
    console.error(
      `${COVERAGE_VARIABLE} is empty in the extension host — the bundle under test was not the instrumented one`,
    );
    return;
  }
  mkdirSync(dir, { recursive: true });
  writeFileSync(resolve(dir, `coverage-${process.pid}.json`), JSON.stringify(table));
}

const collectInto = process.env[COVERAGE_DIR_ENV];
if (collectInto !== undefined && collectInto !== "") {
  process.on("exit", () => {
    dump(collectInto);
  });
}
