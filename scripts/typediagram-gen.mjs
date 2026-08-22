#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { OUT_RUST, OUT_TS, TD_PATH } from "./typediagram-gen/paths.mjs";
import { postprocess } from "./typediagram-gen/rust-postprocess.mjs";
import { postprocessTs, TS_HEADER, tsImports } from "./typediagram-gen/ts-postprocess.mjs";

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

// [BUILD-GEN-IDEMPOTENT] Writes `contents` to `path` only when it would
// change the file, and reports which happened.
//
// The Rust output is a *source file* of deslop-core. Cargo fingerprints
// local sources by mtime, so rewriting it with byte-identical content
// still invalidates deslop-core and, through it, every dependent crate
// and all ~200 release test binaries. This generator is a prerequisite
// of `fmt`, `lint`, `test`, `test-shard`, `coverage` and `_vsix-build`,
// so an unconditional write discarded the release build CI had just
// cached on every one of those targets — measured at 0.13s for a no-op
// workspace rebuild versus 1m30s after one generator run, and ~20
// minutes with `--all-targets`, which is what cancelled all four Rust
// shards on run 32549706011 despite an exact cache hit.
//
// Compares content rather than trusting a timestamp: a clobbered or
// half-written file still differs, so it is still rewritten.
// `scripts/typediagram-gen.test.mjs` pins both halves.
function writeIfChanged(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  try {
    if (readFileSync(path, "utf8") === contents) return "unchanged";
  } catch {
    // No readable file yet (fresh checkout, or a partial write) — fall
    // through and generate it.
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
