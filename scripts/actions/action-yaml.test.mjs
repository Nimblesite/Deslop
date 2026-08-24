// Proves the composite-action step scanner (action-yaml.mjs) actually finds
// the bodies the contract checks measure. Both the shell-injection gate and
// the branch-executed guard proof are only as good as this scanner: one that
// silently returned nothing would declare an injected action.yml clean.
// Spec: release.md [ACTION-TESTS]. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { leadingSpaces, mappingValues, runBodies, stepBody, valuesAfter } from "./action-yaml.mjs";

/** A composite action with a block body, a `uses:` step, and a one-line body. */
const ACTION = [
  "runs:",
  "  using: composite",
  "  steps:",
  "    - name: Guard",
  "      if: inputs.diff == '-'",
  "      shell: bash",
  "      run: |",
  '        printf \'origin/%s\' "${BASE_REF:-main}"',
  "        exit 2",
  "      env:",
  "        BASE_REF: ${{ github.base_ref }}",
  "    - name: Restore the cache",
  "      uses: actions/cache/restore@0000000000000000000000000000000000000000",
  "      with:",
  "        path: .deslop/cache",
  "    - name: Resolve",
  "      shell: bash",
  '      run: node "${GITHUB_ACTION_PATH}/resolve.mjs" "${RUNNER_OS}"',
  "",
].join("\n");

/** The guard step's name in the fixture above. */
const GUARD = "Guard";

/** The `uses:` step's name — it carries no shell body. */
const CACHE = "Restore the cache";

test("every run step is found, and `uses:` steps are not mistaken for one", () => {
  assert.deepEqual(
    runBodies(ACTION).map((step) => step.name),
    [GUARD, "Resolve"],
    `a ${CACHE} entry runs a published action, not a script`,
  );
});

test("a block body is dedented to column zero so bash reads it as a script", () => {
  assert.equal(stepBody(ACTION, GUARD), 'printf \'origin/%s\' "${BASE_REF:-main}"\nexit 2');
});

test("a one-line run value is a body too, not an empty one", () => {
  assert.equal(stepBody(ACTION, "Resolve"), 'node "${GITHUB_ACTION_PATH}/resolve.mjs" "${RUNNER_OS}"');
});

test("the reported line is the 1-based line the run key sits on", () => {
  const guard = runBodies(ACTION).find((step) => step.name === GUARD);
  assert.equal(guard.line, 7, "line 7 is `run: |` in the fixture above");
});

test("a step's env block is not swallowed into its body", () => {
  assert.ok(
    !stepBody(ACTION, GUARD).includes("${{"),
    "reading past the body into `env:` would report every step as injected",
  );
});

test("a `${{ }}` expression inside a body is visible to the caller", () => {
  const injected = ACTION.replace('"${BASE_REF:-main}"', '"${{ github.base_ref }}"');
  assert.equal(
    stepBody(injected, GUARD).includes("${{"),
    true,
    "the gate that rejects shell injection reads exactly this string",
  );
});

test("a step that lost its name is not silently reported as a body", () => {
  assert.throws(() => stepBody(ACTION, "No Such Step"), /lost its "No Such Step" run step/);
});

test("leadingSpaces measures the indentation the scanner cuts", () => {
  assert.equal(leadingSpaces("    - name: Guard"), 4);
  assert.equal(leadingSpaces(""), 0);
});

test("the real action.yml parses into steps with non-empty bodies", () => {
  const steps = runBodies(readFileSync("action.yml", "utf8"));
  assert.ok(steps.length >= 2, `only ${steps.length} run steps were found in action.yml`);
  for (const step of steps) {
    assert.ok(step.body.trim().length > 0, `${step.name} produced an empty body`);
  }
});

// A build matrix repeats one key per leg. The release publish contract reads
// the platform list this way, so a scanner that returned nothing would let
// the platform list and the matrix silently disagree.
const MATRIX = [
  "    strategy:",
  "      matrix:",
  "        include:",
  "          - os: ubuntu-latest",
  "            vsix_target: linux-x64",
  "          - os: macos-latest",
  "            vsix_target: darwin-arm64",
  "          - os: windows-latest",
  "            vsix_target: win32-x64",
  "  vsix_target_note: not-a-target",
].join("\n");

test("every value of a repeated mapping key is collected, in file order", () => {
  assert.deepEqual(mappingValues(MATRIX, "vsix_target"), [
    "linux-x64",
    "darwin-arm64",
    "win32-x64",
  ]);
});

test("a longer key that merely starts with the wanted one is not collected", () => {
  assert.ok(!mappingValues(MATRIX, "vsix_target").includes("not-a-target"));
});

test("a key that appears nowhere yields nothing rather than throwing", () => {
  assert.deepEqual(mappingValues(MATRIX, "no_such_key"), []);
});

test("a declared-but-empty value is not reported as a value", () => {
  assert.deepEqual(mappingValues("  vsix_target:\n  vsix_target: linux-x64", "vsix_target"), [
    "linux-x64",
  ]);
});

// The shared line scanner returns the tail untrimmed: the pinned-ref check
// slices a version off the front of it, and trimming here would hide a value
// that is nothing but spaces from the callers that reject one.
test("the tail of every matching line is returned, indentation ignored", () => {
  assert.deepEqual(valuesAfter(MATRIX, "vsix_target:"), [
    " linux-x64",
    " darwin-arm64",
    " win32-x64",
  ]);
});

test("a line whose prefix appears later than the start is not matched", () => {
  assert.deepEqual(valuesAfter("run: echo vsix_target: linux-x64", "vsix_target:"), []);
});
