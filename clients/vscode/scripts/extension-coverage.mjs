// [VSIX-TESTING-COVERAGE] Extension-host coverage run with the out/ entry point.
//
// The packaged extension activates from dist/extension.js (the esbuild
// bundle), but the c8 gate in .vscode-test.mjs measures the tsc output under
// out/**. Run as-is, activate() executes only inside the bundle, so its
// coverage is never attributed to the measured files — extension.ts reads as
// ~72% no matter how much the E2E suite exercises the live extension.
// Including dist/ in the c8 include is not an option: c8 filters scripts
// before sourcemap remap, so the bundle's map would drag pino and
// vscode-languageclient sources into total.lines.pct, and istanbul cannot
// soundly merge esbuild-mapped and tsc-mapped entries for the same .ts file.
//
// So for the coverage run only, point "main" at out/extension.js — the host
// then activates the exact files c8 measures — and restore package.json
// afterwards. `make _vsix-test` still runs the full suite against the
// production dist bundle, so the shipped artifact stays E2E-validated.

import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { vsixRoot } from "./coverage-paths.mjs";

const packageJsonPath = resolve(vsixRoot, "package.json");
const originalText = readFileSync(packageJsonPath, "utf8");
const manifest = JSON.parse(originalText);
if (manifest.main !== "./dist/extension.js") {
  console.error(
    `package.json main is "${manifest.main}", expected "./dist/extension.js" — refusing to swap`,
  );
  process.exit(1);
}

manifest.main = "./out/extension.js";
writeFileSync(packageJsonPath, `${JSON.stringify(manifest, null, 2)}\n`);
let status = 1;
try {
  const result = spawnSync("npx", ["vscode-test", "--coverage"], {
    cwd: vsixRoot,
    stdio: "inherit",
  });
  status = result.status ?? 1;
} finally {
  writeFileSync(packageJsonPath, originalText);
}
process.exit(status);
