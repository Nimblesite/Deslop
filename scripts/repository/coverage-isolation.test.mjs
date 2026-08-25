// Coverage-isolation contract. [CI-COVERAGE-ISOLATION]
//
// `cargo llvm-cov` collects and reports as two commands, and neither one
// deletes what the previous run left behind. The raw `.profraw` profiles stay,
// and so do the instrumented object files. `report` maps the merged profile
// against every object it can find, so an object from an earlier build hands it
// an older line table for a source file that has since changed — and every line
// in that table is unexecuted, because nothing ran that build.
//
// Measured on this repository: `crates/deslop-lsp/src/app.rs` is 193 lines and
// 99.5% covered; one stale object made it read as 362 lines and 53.0%, dragging
// `deslop-lsp` to 85.1% and failing the release gate on a tree that was fine.
// It fails the other way too — a stale object whose lines did execute credits
// coverage nobody measured.
//
// `--profraw-only` does not fix it: the stale *object* carries the bogus line
// table. These tests hold `coverage-run` to a full `clean --workspace`, before
// collection, every time. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";

import { recipeBlocks } from "../lib/makefile.mjs";

/// The target that collects coverage, and the two commands whose order is the
/// whole contract.
const COLLECT_TARGET = "coverage-run";
const CLEAN_COMMAND = "cargo llvm-cov clean --workspace";
const COLLECT_COMMAND = "cargo llvm-cov --release --workspace --all-targets";

/// The narrowing that reads like the same fix and is not: it deletes the
/// profiles and leaves the object that carries the stale line table.
const PROFILES_ONLY_FLAG = "--profraw-only";

/// The recipe body of the collection target, as one string.
function collectRecipe() {
  const blocks = recipeBlocks(COLLECT_TARGET);
  assert.equal(
    blocks.length,
    1,
    `Makefile must declare exactly one \`${COLLECT_TARGET}\` recipe; found ${blocks.length}`,
  );
  return blocks[0].body;
}

test("[CI-COVERAGE-ISOLATION] collection drops this workspace's previous artifacts first", () => {
  const body = collectRecipe();
  const cleanAt = body.indexOf(CLEAN_COMMAND);
  const collectAt = body.indexOf(COLLECT_COMMAND);
  assert.ok(
    cleanAt >= 0,
    `${COLLECT_TARGET} must run \`${CLEAN_COMMAND}\`; without it, objects from an earlier build ` +
      "contribute an older line table and the reported percentage is not a measurement",
  );
  assert.ok(collectAt >= 0, `${COLLECT_TARGET} no longer runs \`${COLLECT_COMMAND}\``);
  assert.ok(
    cleanAt < collectAt,
    "the clean must precede collection — cleaning afterwards deletes the profiles this run just wrote",
  );
});

test("[CI-COVERAGE-ISOLATION] the clean is never narrowed to the profiles", () => {
  assert.ok(
    !collectRecipe().includes(PROFILES_ONLY_FLAG),
    `${PROFILES_ONLY_FLAG} deletes the raw profiles and keeps the stale object that carries the ` +
      "wrong line table; it reads as the same fix and measures the same wrong number",
  );
});

test("[CI-COVERAGE-ISOLATION] reporting stays a separate command over the cleaned collection", () => {
  const blocks = recipeBlocks("coverage-report");
  assert.equal(blocks.length, 1, "Makefile must declare exactly one `coverage-report` recipe");
  assert.ok(
    blocks[0].body.includes("cargo llvm-cov") && blocks[0].body.includes("report"),
    "coverage-report must run `cargo llvm-cov ... report`; the split is what makes the clean necessary",
  );
  assert.ok(
    !blocks[0].body.includes("clean"),
    "coverage-report must never clean — it runs after collection, and a clean there discards the run",
  );
});
