// [VSIX-TESTING-COVERAGE] Proves every declared VSIX coverage floor is
// actually enforced, and every enforcer names a floor that exists.
//
// `coverage-thresholds.json` carried a `projects` block declaring
// `clients/vscode: 95` that NOTHING read — a 95% VSIX floor on the books over
// code with no coverage channel at all, while the 472 extension-host tests
// behind it were not even executing. A threshold nobody reads is worse than no
// threshold: it reports safety that was never measured. These assertions make
// that state unrepresentable in both directions.
//
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { COVERAGE_GATES, loadThresholds } from "./coverage-paths.mjs";

/// The surfaces that must stay gated. Pinned by name so deleting a gate is a
/// test failure rather than a silently smaller set.
const REQUIRED_GATES = ["--extension", "--webview"];
/// Floors live under this key in coverage-thresholds.json.
const FLOOR_SECTION = "vsix";
/// A floor is a line percentage; anything outside this cannot be one.
const MIN_FLOOR = 0;
const MAX_FLOOR = 100;

const floors = () => loadThresholds()[FLOOR_SECTION] ?? {};

test("every surface that must be gated still has a gate", () => {
  const gates = Object.keys(COVERAGE_GATES).sort();
  assert.deepEqual(
    gates,
    [...REQUIRED_GATES].sort(),
    "a coverage gate was added or removed without updating this contract",
  );
});

test("every gate names a floor that exists and is a real percentage", () => {
  const declared = floors();
  for (const [flag, gate] of Object.entries(COVERAGE_GATES)) {
    const value = declared[gate.thresholdKey];
    assert.notEqual(
      value,
      undefined,
      `${flag} enforces .${FLOOR_SECTION}.${gate.thresholdKey}, which is missing — the gate would exit 1 on every run`,
    );
    assert.ok(
      Number.isFinite(Number(value)) && Number(value) > MIN_FLOOR && Number(value) <= MAX_FLOOR,
      `.${FLOOR_SECTION}.${gate.thresholdKey} is ${JSON.stringify(value)}, not a percentage`,
    );
  }
});

test("no declared floor goes unenforced", () => {
  const claimed = new Set(Object.values(COVERAGE_GATES).map((gate) => gate.thresholdKey));
  const orphans = Object.keys(floors()).filter((key) => !claimed.has(key));
  assert.deepEqual(
    orphans,
    [],
    `these floors are declared but no gate reads them, so they enforce nothing: ${orphans.join(", ")}`,
  );
});

test("each gate reads its own coverage directory", () => {
  const dirs = Object.values(COVERAGE_GATES).map((gate) => gate.coverageDir);
  assert.equal(
    new Set(dirs).size,
    dirs.length,
    "two gates read the same coverage directory, so one is measuring the other's surface",
  );
});
