// Duplication-gate provenance contract. [CI-DESLOP]
//
// The gate that measures this repository must be the detector this tree
// builds — never a published release, and never the Marketplace action, which
// downloads one. A gate running last month's binary reports last month's
// percentage: a branch that changes detection would be measured by a detector
// that predates the change, so a real regression passes and a real improvement
// reads as a breach. These tests assert the wiring that keeps the gate on the
// branch's own `target/release/deslop`. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const makefile = readFileSync(resolve(repoRoot, "Makefile"), "utf8").split("\n");
const ciWorkflow = readFileSync(resolve(repoRoot, ".github/workflows/ci.yml"), "utf8").split("\n");

// The one workflow allowed to install a published CLI: its whole purpose is
// proving the Marketplace action installs and runs the release a consumer
// pins. It scans the examples/ fixtures, never this repository's own tree.
const PUBLISHED_CLI_EXEMPT = "action-selftest.yml";

// The `uses:` value a workflow line carries, normalised so a step written as
// a list item (`- uses: ./`) and one written as a plain mapping key read the
// same. Line-exact string work, never a pattern match over the YAML.
function stepValue(line) {
  const trimmed = line.trim();
  return trimmed.startsWith("- ") ? trimmed.slice(2).trim() : trimmed;
}

// Recipe lines of a make target: everything from the target line up to the
// next line that starts in column 0. Line-exact, no pattern matching.
function recipe(target) {
  const start = makefile.findIndex((line) => line.startsWith(`${target}:`));
  assert.ok(start >= 0, `Makefile no longer declares a \`${target}\` target`);
  const rest = makefile.slice(start + 1);
  const end = rest.findIndex((line) => line.length > 0 && !line.startsWith("\t") && !line.startsWith(" "));
  return { header: makefile[start], body: (end < 0 ? rest : rest.slice(0, end)).join("\n") };
}

test("[CI-DESLOP] the gate runs the binary this workspace builds", () => {
  const { header, body } = recipe("dup-gate");
  assert.equal(
    header,
    "dup-gate: build",
    "dup-gate must depend on `build`, so the gate can never measure with a stale or absent binary",
  );
  assert.ok(
    body.includes("./target/release/deslop"),
    "dup-gate must invoke ./target/release/deslop — the detector built from this tree",
  );
  assert.ok(
    !body.includes("Nimblesite/Deslop") && !body.includes("brew ") && !body.includes("scoop "),
    "dup-gate must never reach for a packaged or published deslop; those are previous releases",
  );
});

test("[CI-DESLOP] that binary is compiled from this tree, not downloaded", () => {
  const { body } = recipe("build");
  assert.ok(
    body.includes("cargo build --release --workspace"),
    "make build must compile the workspace; the gate's binary has to come from the branch's sources",
  );
  assert.ok(
    !body.includes("releases/download") && !body.includes("gh release download"),
    "make build must never fetch a release archive — a downloaded CLI is the previous release",
  );
});

test("[CI-DESLOP] CI builds first, then gates with what it built", () => {
  const buildAt = ciWorkflow.findIndex((line) => line.trim() === "run: make build");
  const gateAt = ciWorkflow.findIndex((line) => line.trim() === "run: make dup-gate");
  assert.ok(buildAt >= 0, "ci.yml no longer runs `make build`");
  assert.ok(gateAt >= 0, "ci.yml no longer runs `make dup-gate`; the duplication gate is the CLI, not an action");
  assert.ok(gateAt > buildAt, "the duplication gate must run after the build, so it measures the freshly built binary");
});

test("[CI-DESLOP] no workflow gates this repository with a published CLI", () => {
  const workflows = readdirSync(resolve(repoRoot, ".github/workflows"));
  const offenders = workflows.flatMap((name) => {
    if (name === PUBLISHED_CLI_EXEMPT) return [];
    const lines = readFileSync(resolve(repoRoot, ".github/workflows", name), "utf8").split("\n");
    return lines
      .map((line, index) => ({ text: stepValue(line), at: index + 1 }))
      .filter(({ text }) => text.startsWith("uses: Nimblesite/Deslop@") || text === "uses: ./")
      .map(({ text, at }) => `${name}:${at}: ${text}`);
  });
  assert.deepEqual(
    offenders,
    [],
    `the Deslop action installs a published release; this repository's own checks must run its own binary:\n  ${offenders.join("\n  ")}`,
  );
});
