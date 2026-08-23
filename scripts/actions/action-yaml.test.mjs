// Proves the composite-action step scanner (action-yaml.mjs) actually finds
// the bodies the contract checks measure. Both the shell-injection gate and
// the branch-executed guard proof are only as good as this scanner: one that
// silently returned nothing would declare an injected action.yml clean.
// Spec: release.md [ACTION-TESTS]. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { leadingSpaces, runBodies, stepBody } from "./action-yaml.mjs";

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
