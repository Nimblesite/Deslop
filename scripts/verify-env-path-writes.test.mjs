// Proves the PATH/env injection lint (verify-env-path-writes.mjs) actually
// rejects a caller-influenced write and passes a constant one — a silently
// broken gate is worse than no gate. Spec: release.md [ACTION-ENVPATH].
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { basename } from "node:path";

import {
  envWriteViolations,
  expansions,
  lintTargets,
  redirectSink,
  writtenFragments,
} from "./verify-env-path-writes.mjs";

// The exact line CodeQL alert 25 reported against action.yml, and the constant
// that replaced it.
const TAINTED_PATH_WRITE = '        echo "${RUNNER_TEMP}/deslop/${STAGE}" >> "${GITHUB_PATH}"';
const CONSTANT_PATH_WRITE = '        echo "${RUNNER_TEMP}/deslop/bin" >> "${GITHUB_PATH}"';

test("a step output written to GITHUB_PATH is rejected, naming the sink and the variable", () => {
  const violations = envWriteViolations(TAINTED_PATH_WRITE, "action.yml");
  assert.deepEqual(violations, [{ file: "action.yml", line: 1, sink: "GITHUB_PATH", name: "STAGE" }]);
});

test("a constant built from runner-owned variables is accepted", () => {
  assert.deepEqual(envWriteViolations(CONSTANT_PATH_WRITE, "action.yml"), []);
});

test("a bare $NAME expansion is caught, not only the braced form", () => {
  const violations = envWriteViolations('echo "$INSTALL_DIR" >> "$GITHUB_PATH"', "w.yml");
  assert.equal(violations[0].name, "INSTALL_DIR");
});

test("a GitHub expression written into GITHUB_ENV is reported whole", () => {
  const violations = envWriteViolations('echo "V=${{ inputs.version }}" >> "$GITHUB_ENV"', "w.yml");
  assert.deepEqual(violations, [
    { file: "w.yml", line: 1, sink: "GITHUB_ENV", name: "${{ inputs.version }}" },
  ]);
});

test("a tainted line inside a redirect group is caught though the redirect itself is clean", () => {
  const source = ['          {', '            echo "CC=${TOOLCHAIN}"', '          } >> "$GITHUB_ENV"'].join("\n");
  const violations = envWriteViolations(source, "release.yml");
  assert.deepEqual(violations, [{ file: "release.yml", line: 3, sink: "GITHUB_ENV", name: "TOOLCHAIN" }]);
});

test("a redirect group of literals is accepted", () => {
  const source = ['          {', '            echo "CC=aarch64-linux-gnu-gcc"', '          } >> "$GITHUB_ENV"'].join("\n");
  assert.deepEqual(envWriteViolations(source, "release.yml"), []);
});

test("a redirect group that was never opened is a hard error, not a silent pass", () => {
  assert.throws(
    () => envWriteViolations(['            echo "CC=x"', '          } >> "$GITHUB_ENV"'].join("\n"), "w.yml"),
    (error) => error.message.includes("never opened"),
  );
});

test("command substitution and a literal $$ are not mistaken for expansions", () => {
  assert.deepEqual(expansions('"$(cygpath -u "${SYSTEMROOT}")/System32/tar.exe"'), ["SYSTEMROOT"]);
  assert.deepEqual(expansions("printf '%s' $$"), []);
});

test("GITHUB_OUTPUT is out of scope — it is not replayed as PATH or environment", () => {
  assert.equal(redirectSink('echo "version=${tag#v}" >> "$GITHUB_OUTPUT"'), "");
  assert.deepEqual(envWriteViolations('echo "version=${TAG}" >> "$GITHUB_OUTPUT"', "w.yml"), []);
});

test("only the fragment before the redirect is inspected, so the sink name itself is not a violation", () => {
  assert.deepEqual(writtenFragments([CONSTANT_PATH_WRITE], 0), ['        echo "${RUNNER_TEMP}/deslop/bin" ']);
});

test("the lint covers action.yml and every workflow", () => {
  const targets = lintTargets().map((target) => basename(target));
  assert.ok(targets.includes("action.yml"), "action.yml is the composite action under Marketplace listing");
  for (const workflow of ["ci.yml", "release.yml", "action-selftest.yml", "codeql.yml", "deploy-pages.yml"]) {
    assert.ok(targets.includes(workflow), `${workflow} is not covered by the env/PATH lint`);
  }
});

test("every file in this repo writes only runner-owned constants into PATH and env", () => {
  const violations = lintTargets().flatMap((target) =>
    envWriteViolations(readFileSync(target, "utf8"), target),
  );
  assert.deepEqual(violations, [], "a caller-influenced PATH/env write landed in a workflow");
});
