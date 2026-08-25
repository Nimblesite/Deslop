// [VSIX-TESTING-COVERAGE-RESTORE] Black-box contract for the extension-host coverage
// script: a failed clean recompile must fail the command, and the message must
// name the tree that was left instrumented.
//
// `extension-coverage.test.mjs` pins the decision table on its own. This file
// pins the wiring around it — that the script really reads the restore status,
// really routes it through the decision, and really names `out/**` — by
// running `scripts/extension-coverage.mjs` as a process. Nothing here reaches
// into the script; it is driven entirely through PATH, its exit code, and its
// stderr.
//
// The run happens inside a sandbox root under `clients/vscode/` so the script's
// own `vsixRoot` resolves there: it deletes and recreates its coverage
// directory on every run, and it must never do that to the real one. Being
// under `clients/vscode/` also keeps `node_modules` resolvable for the
// script's imports.
//
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  cpSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { delimiter, resolve } from "node:path";
import { vsixRoot } from "./coverage-paths.mjs";

/// Sandbox root. Under clients/vscode so the script's vsixRoot lands here.
const SANDBOX = resolve(vsixRoot, ".tmp-extension-coverage-contract");
/// The scripts the sandboxed copy of the entry point imports.
const COPIED_SCRIPTS = ["extension-coverage.mjs", "coverage-paths.mjs"];
/// Exit status the stubbed clean recompile fails with.
const RESTORE_EXIT = 3;
/// The tools the script shells out to, every one stubbed.
const STUBBED_TOOLS = ["npm", "node", "npx"];
/// Where the stub records how many times it has been called.
const CALL_LOG = "calls.log";
/// The script must fail, not pass, when the tree is left instrumented.
const EXPECTED_EXIT = 1;
/// The npm script that copies the fixtures the suites open.
const STAGE_FIXTURES = "stage-fixtures";

/// Writes an executable stub for `tool` into `binDir`.
///
/// `npm run compile` is the clean recompile the script runs in its `finally`.
/// The stub lets the first one through — that is collection's own compile —
/// and fails the second, which is the restore. Every other tool succeeds, so
/// collection reaches the end and the restore failure is the *only* thing
/// wrong with the run.
function writeStub(binDir, tool) {
  const script = resolve(binDir, tool);
  writeFileSync(
    script,
    [
      "#!/bin/sh",
      `log="$STUB_DIR/${CALL_LOG}"`,
      `echo "${tool} $*" >> "$log"`,
      `if [ "${tool}" = "npm" ] && [ "$1" = "run" ] && [ "$2" = "compile" ]; then`,
      `  count=$(grep -c "^npm run compile" "$log")`,
      `  if [ "$count" -ge 2 ]; then exit ${RESTORE_EXIT}; fi`,
      "fi",
      "exit 0",
      "",
    ].join("\n"),
  );
  chmodSync(script, 0o755);
}

/// Runs the sandboxed script with every shelled-out tool stubbed.
function runSandboxed() {
  rmSync(SANDBOX, { recursive: true, force: true });
  const scripts = resolve(SANDBOX, "scripts");
  const binDir = resolve(SANDBOX, "bin");
  mkdirSync(scripts, { recursive: true });
  mkdirSync(binDir, { recursive: true });
  writeFileSync(resolve(binDir, CALL_LOG), "");
  for (const name of COPIED_SCRIPTS) {
    cpSync(resolve(vsixRoot, "scripts", name), resolve(scripts, name));
  }
  for (const tool of STUBBED_TOOLS) writeStub(binDir, tool);

  return spawnSync(process.execPath, [resolve(scripts, "extension-coverage.mjs")], {
    encoding: "utf8",
    env: {
      ...process.env,
      STUB_DIR: binDir,
      PATH: `${binDir}${delimiter}${process.env.PATH ?? ""}`,
    },
  });
}

/// Reads the stub call log written by the sandboxed run.
function callLog() {
  return readFileSync(resolve(SANDBOX, "bin", CALL_LOG), "utf8");
}

// [VSIX-TESTING-COVERAGE] `npx vscode-test` bypasses the `pretest` hook, so the
// script owns fixture staging itself. Without it the suites open
// `out/test/fixtures/**` files that `tsc` never produces: green on a tree that
// ran the suite before, red on a clean checkout.
test("the suite is handed staged fixtures, after instrumentation", (t) => {
  t.after(() => rmSync(SANDBOX, { recursive: true, force: true }));
  runSandboxed();
  const calls = callLog().split("\n");
  const staged = calls.findIndex((line) => line === `npm run ${STAGE_FIXTURES}`);
  const instrumented = calls.findIndex((line) => line.startsWith("node "));
  const suite = calls.findIndex((line) => line.startsWith("npx vscode-test"));

  assert.ok(staged >= 0, `the run never staged the fixtures:\n${callLog()}`);
  assert.ok(suite >= 0, `the run never reached the suite:\n${callLog()}`);
  assert.ok(
    instrumented >= 0 && instrumented < staged,
    `instrumentation must run before staging, or it can rewrite what was ` +
      `staged:\n${callLog()}`,
  );
  assert.ok(
    staged < suite,
    `the fixtures must be on disk before the suite opens them:\n${callLog()}`,
  );
});

test("a failed clean recompile fails the command and names the instrumented tree", (t) => {
  t.after(() => rmSync(SANDBOX, { recursive: true, force: true }));
  const run = runSandboxed();

  assert.equal(
    run.status,
    EXPECTED_EXIT,
    `collection succeeded and only the restore failed, so the command must still fail — ` +
      `exiting 0 here ships instrumented modules. stderr:\n${run.stderr}`,
  );
  assert.match(
    run.stderr,
    /still staged/,
    `the failure must say the tree was left behind: ${run.stderr}`,
  );
  assert.ok(
    run.stderr.includes(resolve(SANDBOX, "out")),
    `the failure must name out/**, the tree that was instrumented, not the coverage ` +
      `report directory: ${run.stderr}`,
  );
  assert.ok(
    run.stderr.includes(String(RESTORE_EXIT)),
    `the failure must carry the recompile's own status: ${run.stderr}`,
  );
  assert.doesNotMatch(
    run.stdout,
    /line coverage by file/,
    "a run that left instrumented output behind must not also print a coverage report",
  );
});
