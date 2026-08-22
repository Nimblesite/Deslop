// Generator idempotence contract. [CI-RELEASE-BUILD] [BUILD-GEN-IDEMPOTENT]
//
// `crates/deslop-core/src/wire_generated.rs` is a *source file* of
// deslop-core — gitignored, but compiled like any other module. Cargo
// fingerprints local sources by mtime, so rewriting that file with
// byte-identical content still invalidates deslop-core and, through it,
// every crate and every one of the ~200 release test binaries that
// depend on it.
//
// `typediagram-gen` is a prerequisite of `fmt`, `lint`, `test`,
// `test-shard`, `coverage` and `_vsix-build`. An unconditional write
// therefore threw away the release build CI had just cached, on every
// one of those targets. Measured on this tree: a no-op
// `cargo build --release --workspace` is 0.13s; the same build after one
// generator run is 1m30s. With `--all-targets` that is the ~20 minutes
// that cancelled all four Rust shards on CI run 32549706011 at their
// 20-minute cap — with an *exact* cache hit restored moments earlier.
//
// These tests pin the only property that makes "compile once in `build`,
// restore in the shards" work: regenerating unchanged models must not
// touch the outputs. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { OUT_RUST, OUT_TS } from "./typediagram-gen/paths.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const generator = fileURLToPath(new URL("./typediagram-gen.mjs", import.meta.url));

/** Runs the generator the way the Makefile does. */
function generate() {
  execFileSync("node", [generator], { cwd: repoRoot, stdio: "pipe" });
}

/** The mtime cargo fingerprints this path by, in milliseconds. */
function mtimeMs(path) {
  return statSync(path).mtimeMs;
}

// Both generated files are dated into the past first, so a rewrite is
// unmissable: a generator that touches the file moves its mtime forward
// by roughly a day, far outside any filesystem timestamp granularity.
const BACKDATE_SECONDS = 24 * 60 * 60;

function backdate(path) {
  const past = new Date(Date.now() - BACKDATE_SECONDS * 1000);
  utimesSync(path, past, past);
  return mtimeMs(path);
}

test("regenerating unchanged models leaves the Rust module's mtime alone", () => {
  generate();
  const backdated = backdate(OUT_RUST);

  generate();

  assert.equal(
    mtimeMs(OUT_RUST),
    backdated,
    `${OUT_RUST} was rewritten despite identical content — this invalidates ` +
      "deslop-core and forces a full workspace + test-binary recompile",
  );
});

test("regenerating unchanged models leaves the TypeScript module's mtime alone", () => {
  generate();
  const backdated = backdate(OUT_TS);

  generate();

  assert.equal(
    mtimeMs(OUT_TS),
    backdated,
    `${OUT_TS} was rewritten despite identical content — this invalidates the ` +
      "esbuild/tsc caches the VSIX build restores",
  );
});

test("a stale generated module is still rewritten", () => {
  generate();
  const generated = readFileSync(OUT_RUST, "utf8");
  writeFileSync(OUT_RUST, "// clobbered — not what the .td describes\n", "utf8");

  generate();

  assert.equal(
    readFileSync(OUT_RUST, "utf8"),
    generated,
    "skipping the write must never leave a stale module in place",
  );
});
