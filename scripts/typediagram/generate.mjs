#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { OUT_RUST, OUT_TS, TD_PATH } from "./paths.mjs";
import { postprocess } from "./rust-postprocess.mjs";
import { postprocessTs, TS_HEADER, tsImports } from "./ts-postprocess.mjs";

function runTypediagram(target) {
  // On Windows the global npm install exposes `typediagram` as a `.cmd`
  // shim; execFileSync only resolves that through a shell (it does not
  // append PATHEXT itself). Unix resolves the bare binary directly.
  const stdout = execFileSync("typediagram", ["--to", target, TD_PATH], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  return stdout;
}

function generateTs() {
  const raw = runTypediagram("typescript");
  const body = postprocessTs(raw);
  const imports = tsImports(body);
  const importBlock = imports.length > 0 ? `${imports.join("\n")}\n\n` : "";
  return `${TS_HEADER}\n${importBlock}${body}`;
}

// [BUILD-GEN-IDEMPOTENT] Writes only when the content would change.
//
// The Rust output is a source file of deslop-core, and cargo fingerprints
// local sources by mtime — so rewriting it with byte-identical content
// still invalidates deslop-core and every crate and test target below it.
// This generator is a prerequisite of `fmt`, `lint`, `test`, `test-shard`,
// `coverage` and `_vsix-build`, so an unconditional write discarded the
// release build the previous target had just cached. Measured: a no-op
// workspace rebuild is 0.13s, and 1m30s after one generator run.
//
// Compares content rather than trusting a timestamp, so a clobbered or
// half-written file is still regenerated.
function writeIfChanged(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  try {
    if (readFileSync(path, "utf8") === contents) return "unchanged";
  } catch {
    // No readable file yet (fresh checkout, or a partial write).
  }
  writeFileSync(path, contents, "utf8");
  return "wrote";
}

function main() {
  const rust = postprocess(runTypediagram("rust"));
  process.stdout.write(`typediagram-gen: ${writeIfChanged(OUT_RUST, rust)} ${OUT_RUST}\n`);

  const ts = generateTs();
  process.stdout.write(`typediagram-gen: ${writeIfChanged(OUT_TS, ts)} ${OUT_TS}\n`);
}

main();
