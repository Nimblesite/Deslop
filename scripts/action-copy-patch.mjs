// The duplicated-code patch both diff-gate proofs run against
// ([ACTION-GATE], [METRICS-DIFF-SCOPE]).
//
// The gate's breaching leg needs a diff whose added lines are duplicated
// by construction: a verbatim copy of an existing fixture file. Both the
// hosted-runner leg in .github/workflows/action-selftest.yml and the
// branch-built proof in scripts/test-action-diff-gate.mjs need exactly
// that patch, so it is built here once. A shell twin of this that counted
// lines with `wc -l` would disagree with this one on a source file with no
// trailing newline — `wc -l` counts terminators, so the hunk header would
// declare one line fewer than the body carries and the parser would refuse
// the patch outright ("hunk body exceeds the counts declared in its
// header"), failing the proof for a reason that has nothing to do with the
// gate.
//
// Usage: node scripts/action-copy-patch.mjs <scanPath> <targetRelative> <patchPath>

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

/**
 * First Rust source under `scanPath`, sorted so the choice is stable —
 * keeps both proofs working if the fixture layout is reorganised.
 *
 * @param {string} scanPath fixture tree to search
 * @returns {string} repo-relative path to the first Rust file
 */
export function firstRustFile(scanPath) {
  const found = execFileSync("find", [scanPath, "-name", "*.rs", "-type", "f"], { encoding: "utf8" })
    .split("\n")
    .filter(Boolean)
    .sort();
  assert.ok(found[0], `no Rust source under ${scanPath} to build a copy patch from`);
  return found[0];
}

/**
 * Copies `sourceRelative` to `targetRelative` and writes the unified diff
 * that adds it, so the patch and the tree agree byte for byte — the
 * verifier refuses a diff that does not.
 *
 * @param {string} sourceRelative fixture file to copy
 * @param {string} targetRelative the new file the patch adds
 * @param {string} patchPath where to write the patch
 * @returns {string} `patchPath`
 */
export function writeCopyPatch(sourceRelative, targetRelative, patchPath) {
  const lines = readFileSync(sourceRelative, "utf8").split("\n");
  const trailingNewline = lines.at(-1) === "";
  const body = trailingNewline ? lines.slice(0, -1) : lines;
  const hunk = [
    `diff --git a/${targetRelative} b/${targetRelative}`,
    "new file mode 100644",
    "--- /dev/null",
    `+++ b/${targetRelative}`,
    `@@ -0,0 +1,${body.length} @@`,
    ...body.map((line) => `+${line}`),
    ...(trailingNewline ? [] : ["\\ No newline at end of file"]),
    "",
  ].join("\n");
  writeFileSync(patchPath, hunk);
  writeFileSync(targetRelative, readFileSync(sourceRelative));
  return patchPath;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [scanPath, targetRelative, patchPath] = process.argv.slice(2);
  assert.ok(
    scanPath && targetRelative && patchPath,
    "usage: node scripts/action-copy-patch.mjs <scanPath> <targetRelative> <patchPath>",
  );
  writeCopyPatch(firstRustFile(scanPath), targetRelative, patchPath);
}
